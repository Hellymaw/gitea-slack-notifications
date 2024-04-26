use axum::body;
use axum::http::request;
use axum::{extract::Json, extract::State, routing::post, Router};
use gitea_webhooks::config_env_var;
use gitea_webhooks::Action;
use gitea_webhooks::MySlackMessage;
use gitea_webhooks::User;
use gitea_webhooks::Webhook;

use serde::Deserialize;
use serde_json::{self, json};
use slack_morphism::prelude::*;

pub mod gitea_webhooks;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug)]
pub struct AppState {
    pub slack_message_cache: HashMap<String, SlackTs>,
    pub slack_user_lookup: HashMap<String, String>,
}

pub type SharedState = Arc<Mutex<AppState>>;

const BIND_ADDRESS: &str = "192.168.0.26:6969";
const GITEA_ADDRESS: &str = "localhost:3000";

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
    println!("webhook: {:?}", payload);

    let ts = {
        let state_data = state.lock().unwrap();
        state_data
            .slack_message_cache
            .get(&payload.pull_request.url)
            .map(|ts| ts.clone())
    };

    let response = post_slack_message(&payload, &ts).await;
    if ts.is_none() {
        if let Ok(response) = response {
            let mut state_data = state.lock().unwrap();
            state_data
                .slack_message_cache
                .entry(payload.pull_request.url.clone())
                .or_insert(response);

            println!("Added new entry: {:?}", state_data.slack_message_cache);
        }
    }
}

async fn post_slack_message(
    message: &Webhook,
    parent: &Option<SlackTs>,
) -> Result<SlackTs, Box<dyn std::error::Error + Send + Sync>> {
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_TOKEN")?.into();
    let token: SlackApiToken = SlackApiToken::new(token_value);
    let session = client.open_session(&token);

    let message = message.into_my_slack().await;
    let message = message.render_template();

    let post_chat_req = if let Some(thread_ts) = parent {
        SlackApiChatPostMessageRequest::new("#aaron-test-channel".into(), message)
            .with_thread_ts(thread_ts.clone())
    } else {
        SlackApiChatPostMessageRequest::new("#aaron-test-channel".into(), message)
    };

    let post_chat_resp = session.chat_post_message(&post_chat_req).await?;

    Ok(post_chat_resp.ts)
}
