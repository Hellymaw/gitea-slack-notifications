use axum::Extension;
use axum::{extract::Json, routing::post, Router};
use gitea_webhooks::Webhook;
use serde_json;
use slack_morphism::prelude::*;
use sqlx::postgres::PgPool;
use tower_http::trace::TraceLayer;
use tracing;
use tracing_appender;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod gitea_webhooks;

const MAX_LOG_FILES: usize = 48;

#[tokio::main]
async fn main() {
    let log_dir = std::env::var("LOG_DIR").unwrap_or("./logs".to_string());
    let log_suffix = std::env::var("LOG_SUFFIX").unwrap_or("gitea_notifs.log".to_string());

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::HOURLY)
        .filename_suffix(&log_suffix)
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
        .expect("Failed to initialise rolling file appender");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    let db_pool = PgPool::connect(&construct_db_connection_string())
        .await
        .unwrap();

    let app = Router::new()
        .route("/", post(post_handler))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(db_pool));

    let bind_addr = std::env::var("BIND_ADDRESS").expect("A binding address is required");
    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn post_handler(db: Extension<PgPool>, Json(payload): Json<serde_json::Value>) {
    tracing::debug!(%payload);

    match serde_json::from_value::<Webhook>(payload) {
        Ok(webhook) => post_repo_payload(webhook, db).await,
        Err(x) => tracing::error!("Error decoding JSON payload into Webhook \"{}\"", x),
    }
}

async fn post_repo_payload(payload: Webhook, db: Extension<PgPool>) {
    let payload = payload.try_deanonymise_emails().await;

    let ts = {
        let rows: Result<Option<(String,)>, sqlx::Error> =
            sqlx::query_as("SELECT ts FROM threads WHERE url = $1")
                .bind(payload.pull_request.url.to_string())
                .fetch_optional(&*db)
                .await;

        match rows {
            Ok(rows) => rows.map(|row| SlackTs::new(row.0)),
            Err(x) => {
                tracing::error!(
                    "Error attempting to retrieve possible timestamp from DB: \"{}\"",
                    x
                );
                None
            }
        }
    };

    let response = payload.post_slack_message(&ts).await;
    if ts.is_none() {
        if let Ok(response) = response {
            let resp = sqlx::query("INSERT INTO threads VALUES ($1, $2)")
                .bind(payload.pull_request.url.as_str())
                .bind(&response.0)
                .execute(&*db)
                .await;

            if let Err(x) = resp {
                tracing::error!(
                    "Error attempting to add a new timestamp to the DB: \"{}\"",
                    x
                )
            } else {
                tracing::info!("Top level Slack Thread created");
            }
        }
    }
}

fn construct_db_connection_string() -> String {
    let pg_password = std::env::var("POSTGRES_PASSWORD").expect("This is a required env var");
    let pg_db = std::env::var("POSTGRES_DB").expect("This is a required env var");

    format!("postgres://postgres:{pg_password}@db/{pg_db}")
}
