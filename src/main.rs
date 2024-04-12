use axum::{extract::Json, extract::State, routing::post, Router};
use gitea_webhooks::{Action, OutgoingWebhook, Review, User, Webhook};
use serde_json;
use slack_morphism::prelude::*;

pub mod gitea_webhooks;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug)]
pub struct AppState {
    pub cache: HashMap<String, SlackTs>,
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
        Ok(webhook) => match webhook.action {
            Action::ReviewRequested {
                requested_reviewer: ref reviewer,
            } => review_requested(&webhook, &reviewer, state).await,
            Action::Reviewed { ref review } => reviewed(&webhook, &review, state).await,
            // Action::Closed => opened(webhook, state).await,
            Action::Opened => opened(&webhook, state).await,
            action => println!("Unhandled action \"{:?}\"", action),
        },
        Err(x) => println!("{}", x),
    }
}

async fn review_requested(payload: &Webhook, reviewer: &User, state: SharedState) {
    let requester = &payload.sender.email;

    let body = format!("{} requested a review from {}", requester, reviewer.email);

    println!("{body}");

    post_repo_payload(payload, &body, state).await;
}

async fn reviewed(payload: &Webhook, review: &Review, state: SharedState) {
    let reviewer = &payload.sender.email;

    let body = format!("{} {:?} the pull-request", reviewer, review);

    println!("{body}");

    post_repo_payload(payload, &body, state).await;
}

async fn opened(payload: &Webhook, state: SharedState) {
    let outgoing = OutgoingWebhook {
        email: payload.sender.email.to_owned(),
        title: "opened PR#".to_owned(),
        body: "".to_owned(),
    };

    let body = serde_json::to_string(&outgoing).unwrap();
    println!("{:?}", body);

    post_repo_payload(payload, &body, state).await;
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

async fn post_repo_payload(payload: &Webhook, body: &str, state: SharedState) {
    let ts = {
        let state_data = state.lock().unwrap();
        state_data
            .cache
            .get(&payload.pull_request.url)
            .map(|ts| ts.clone())
    };

    if let Ok(response) = post_slack_message(payload, ts).await {
        let mut state_data = state.lock().unwrap();
        state_data
            .cache
            .entry(payload.pull_request.url.clone())
            .or_insert(response);

        println!("state_data: {state_data:?}");
    }
}

async fn post_slack_message(
    message: &Webhook,
    parent: Option<SlackTs>,
) -> Result<SlackTs, Box<dyn std::error::Error + Send + Sync>> {
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_TOKEN")?.into();
    let token: SlackApiToken = SlackApiToken::new(token_value);
    let session = client.open_session(&token);

    let message = message.render_template();

    let post_chat_req = if let Some(thread_ts) = parent {
        SlackApiChatPostMessageRequest::new("#aaron-test-channel".into(), message)
            .with_thread_ts(thread_ts)
    } else {
        SlackApiChatPostMessageRequest::new("#aaron-test-channel".into(), message)
    };

    let post_chat_resp = session.chat_post_message(&post_chat_req).await?;

    Ok(post_chat_resp.ts)
}
