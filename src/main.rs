use axum::{extract::Json, extract::State, routing::post, Router};
use gitea_webhooks::{Action, OutgoingWebhook, Review, User, Webhook};
use serde_json;
use slack_morphism::prelude::*;

pub mod gitea_webhooks;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug)]
pub struct AppState {
    pub cache: HashMap<String, u64>,
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
            } => review_requested(&webhook, &reviewer).await,
            Action::Reviewed { ref review } => reviewed(&webhook, &review).await,
            // Action::Closed => opened(webhook, state).await,
            Action::Opened => opened(webhook, state).await,
            action => println!("Unhandled action \"{:?}\"", action),
        },
        Err(x) => println!("{}", x),
    }
}

async fn review_requested(payload: &Webhook, reviewer: &User) {
    let requester = &payload.sender.email;

    let body = format!("{} requested a review from {}", requester, reviewer.email);

    println!("{body}");
}

async fn reviewed(payload: &Webhook, review: &Review) {
    let reviewer = &payload.sender.email;

    let body = format!("{} {:?} the pull-request", reviewer, review);

    println!("{body}");
}

async fn opened(payload: Webhook, state: SharedState) {
    let outgoing = OutgoingWebhook {
        email: payload.sender.email.to_owned(),
        title: "opened PR#".to_owned(),
        body: "".to_owned(),
    };

    {
        let mut state_data = state.lock().unwrap();
        state_data
            .cache
            .insert(payload.pull_request.url, payload.pull_request.id);

        println!("state_data: {state_data:?}");
    }

    let body = serde_json::to_string(&outgoing).unwrap();

    println!("{:?}", body);
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

async fn post_slack_message() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_TOKEN")?.into();
    let token: SlackApiToken = SlackApiToken::new(token_value);
    let session = client.open_session(&token);

    // let message = WelcomeMessageTemplateParams::new("".into());

    let post_chat_req =
        SlackApiChatPostMessageRequest::new("#random".into(), message.render_template());

    let post_chat_resp = session.chat_post_message(&post_chat_req).await?;
    println!("post chat resp: {:#?}", &post_chat_resp);

    Ok(())
}
