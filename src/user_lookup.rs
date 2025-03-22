use slack_morphism::prelude::*;
use sqlx::{postgres::PgQueryResult, PgPool};
use tracing;
use tracing::info;
use tracing::instrument;

use crate::app_state::AppState;
use crate::gitea;
use crate::gitea::GiteaResourcePool;
use crate::slack;

#[derive(Debug, sqlx::FromRow)]
pub struct CachedUser {
    gitea_username: String,
    #[sqlx(try_from = "String")]
    slack_uid: SlackUserId,
}

impl CachedUser {
    // TODO: Better email type
    pub fn new(gitea_username: String, slack_uid: SlackUserId) -> Self {
        Self {
            gitea_username,
            slack_uid,
        }
    }

    pub fn gitea_username(&self) -> &str {
        &self.gitea_username
    }

    pub fn slack_uid(&self) -> &SlackUserId {
        &self.slack_uid
    }

    // NOTE: change the pg result
    #[instrument(skip(db))]
    pub async fn insert_to_database(&self, db: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        let res =
            sqlx::query("INSERT INTO user_lookup(gitea_username, slack_uid) VALUES ($1, $2);")
                .bind(self.gitea_username())
                .bind(&self.slack_uid().0)
                .execute(&*db)
                .await;

        match res {
            Ok(ref res) if res.rows_affected() == 0 => {
                tracing::warn!("Attempted to cache user into databasse, but affected no rows")
            }
            Ok(ref res) if res.rows_affected() > 1 => tracing::warn!(
                "Insert apparently successful. {} rows were affected though!",
                res.rows_affected()
            ),
            Ok(_) => tracing::debug!("User successfully inserted to database"),
            Err(ref e) => tracing::error!("Issue inserting user into database: {e}"),
        }

        res
    }

    #[instrument(skip(db))]
    pub async fn fetch_from_database(
        db: &PgPool,
        gitea_username: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let res = sqlx::query_as(
            "SELECT gitea_username, slack_uid FROM user_lookup WHERE gitea_username = $1;",
        )
        .bind(gitea_username)
        .fetch_optional(&*db)
        .await;

        match res {
            Ok(Some(ref user)) => tracing::debug!("Retrieved user: {user:?} from database"),
            Ok(None) => tracing::info!("User is not in database"),
            Err(ref e) => tracing::error!(%e),
        }

        res
    }

    #[instrument]
    pub async fn fetch_from_external(
        gp: &GiteaResourcePool,
        url: url::Url,
        gitea_username: &str,
    ) -> Result<Self, anyhow::Error> {
        let gitea_user = gitea::user::User::fetch_from_username(gp, url, gitea_username).await?;
        let slack_uid = slack::user::fetch_user_from_email(gitea_user.email().to_owned()).await?;

        Ok(CachedUser::new(
            gitea_user.username().to_owned(),
            slack_uid.id,
        ))
    }

    #[instrument(skip(app))]
    pub async fn fetch(
        app: &AppState,
        url: url::Url,
        gitea_username: &str,
    ) -> Result<Self, anyhow::Error> {
        let user =
            if let Some(user) = Self::fetch_from_database(app.database(), gitea_username).await? {
                user
            } else {
                let user = Self::fetch_from_external(app.gitea(), url, gitea_username).await?;
                let _ = user.insert_to_database(app.database()).await;
                user
            };

        info!("Fetched user: {user:?}");
        Ok(user)
    }
}
