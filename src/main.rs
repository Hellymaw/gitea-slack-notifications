use axum::{extract::Json, extract::State, routing::post, Router};
use gitea_webhooks::Webhook;
use serde_json;
use slack_morphism::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_http::trace::TraceLayer;
use tracing;
use tracing_appender;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod gitea_webhooks;

#[derive(Default, Debug)]
pub struct AppState {
    pub slack_message_cache: HashMap<String, SlackTs>,
    pub slack_user_lookup: HashMap<String, String>,
}

pub type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    let log_dir = std::env::var("LOG_DIR").unwrap_or("./logs".to_string());

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::HOURLY)
        .filename_suffix("gitea_notifs.log")
        .max_log_files(48)
        .build(log_dir)
        .expect("Failed to initialise rolling file appender");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    let state = SharedState::default();

    let app = Router::new()
        .route("/", post(post_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4242").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn post_handler(State(state): State<SharedState>, Json(payload): Json<serde_json::Value>) {
    tracing::debug!(%payload);

    match serde_json::from_value::<Webhook>(payload) {
        Ok(webhook) => post_repo_payload(webhook, state).await,
        Err(x) => tracing::error!("Error decoding JSON payload into Webhook \"{}\"", x),
    }
}

async fn post_repo_payload(payload: Webhook, state: SharedState) {
    let payload = payload.try_deanonymise_emails().await;

    let ts = {
        let state_data = state.lock().unwrap();
        state_data
            .slack_message_cache
            .get(&payload.pull_request.url.as_str().to_string())
            .map(|ts| ts.clone())
    };

    let response = payload.post_slack_message(&ts).await;
    if ts.is_none() {
        if let Ok(response) = response {
            let mut state_data = state.lock().unwrap();
            state_data
                .slack_message_cache
                .entry(payload.pull_request.url.as_str().to_string())
                .or_insert(response);

            tracing::info!("Top level Slack Thread created");
        }
    }
}
