use axum::{extract::Json, extract::State, routing::post, Router};
use gitea_webhooks::Webhook;
use serde_json;
use slack_morphism::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod gitea_webhooks;

#[derive(Default, Debug)]
pub struct AppState {
    pub slack_message_cache: HashMap<String, SlackTs>,
    pub slack_user_lookup: HashMap<String, String>,
}

pub type SharedState = Arc<Mutex<AppState>>;

const BIND_ADDRESS: &str = "192.168.0.26:6969";

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let state = SharedState::default();

    let app = Router::new()
        .route("/", post(post_handler))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn post_handler(State(state): State<SharedState>, Json(payload): Json<serde_json::Value>) {
    match serde_json::from_value::<Webhook>(payload) {
        Ok(webhook) => post_repo_payload(webhook, state).await,
        Err(x) => println!("{}", x),
    }
}

async fn post_repo_payload(payload: Webhook, state: SharedState) {
    let payload = payload.deanonymise_emails().await.unwrap();

    let ts = {
        let state_data = state.lock().unwrap();
        state_data
            .slack_message_cache
            .get(&payload.pull_request.url)
            .map(|ts| ts.clone())
    };

    let response = payload.post_slack_message(&ts).await;
    if ts.is_none() {
        if let Ok(response) = response {
            let mut state_data = state.lock().unwrap();
            state_data
                .slack_message_cache
                .entry(payload.pull_request.url.clone())
                .or_insert(response);
        }
    }
}
