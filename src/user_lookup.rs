use sqlx::{postgres::PgQueryResult, PgPool};
use url::Url;
use slack_morphism::prelude::*;
use tracing;

use crate::slack;
use crate::gitea;

#[derive(Debug, sqlx::FromRow)]
pub struct CachedUser {
    gitea_tag: String,
    #[sqlx(try_from = "String")]
    slack_uid: SlackUserId,
}

impl CachedUser {
    // TODO: Better email type
    pub fn new(gitea_tag: String, slack_uid: SlackUserId) -> Self {
        Self {
            gitea_tag,
            slack_uid
        }
    }

    pub fn gitea_tag(&self) -> &str {
        &self.gitea_tag
    }

    pub fn slack_uid(&self) -> &SlackUserId {
        &self.slack_uid
    }

    // NOTE: change the pg result
    pub async fn insert_to_database(&self, db: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("INSERT INTO user_lookup(gitea_tag, email, slack_uid) VALUES ($1, $2, $3);")
            .bind(self.gitea_tag())
            .bind(&self.slack_uid().0)
            .execute(&*db)
            .await
    }
    
    pub async fn fetch_from_database(db: &PgPool, gitea_tag: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT gitea_tag, slack_uid FROM user_lookup WHERE gitea_tag = $1;")
            .bind(gitea_tag)
            .fetch_optional(&*db)
            .await
    }

    pub async fn fetch_from_external(gitea_tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Pass this in somehow
        let mut url = Url::parse("http://test.com")?;
        
        let gitea_user = gitea::user::User::fetch_from_username(&mut url, gitea_tag).await?;
        let slack_uid = slack::user::fetch_user_from_email(gitea_user.email().to_owned()).await?;

        Ok(CachedUser::new(gitea_tag.to_string(), slack_uid.id))
    }

    pub async fn fetch(db: &PgPool, gitea_tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: start span? Or do spans start on function begin by default?
        
        let user = if let Some(user) = Self::fetch_from_database(db, gitea_tag).await? {
            tracing::debug!("User: {user:?} found in database");
            user
        } else {
            tracing::debug!("User ({gitea_tag:?}) not contained in database, fetching them now...");
            
            let user = Self::fetch_from_external(gitea_tag).await?;
            tracing::debug!("User fetched from external sources. {user:?}");

            tracing::debug!("Attempting to insert user {user:?} into database...");
            match user.insert_to_database(db).await {
                Ok(res) if res.rows_affected() == 0 => tracing::warn!("Attempted to cache user {user:?} into databasse, but affected no rows"),
                Ok(res) if res.rows_affected() > 1 => tracing::warn!("Insert apparently successful. {} rows were affected though!", res.rows_affected()),
                Ok(_) => tracing::debug!("User {user:?} successfully inserted to database"),
                Err(e) => tracing::error!("Issue inserting user into database: {e}"),
            }
            user
        };

        Ok(user)
    }
}
