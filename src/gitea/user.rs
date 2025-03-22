use serde::Deserialize;
use tracing::{self, instrument};

use super::GiteaResourcePool;

#[derive(Deserialize, Debug)]
pub struct User {
    email: String,
    username: String,
}

impl User {
    pub fn new(email: String, username: String) -> Self {
        Self { email, username }
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    #[instrument(skip(gp))]
    pub async fn fetch_from_username(
        gp: &GiteaResourcePool,
        mut url: url::Url,
        username: &str,
    ) -> Result<User, anyhow::Error> {
        const USERS_API_URI: &str = "api/v1/users/";

        // Allows the single server to work for multiple Gitea hosts without configuration
        url.set_path(USERS_API_URI);
        let url = url.join(username)?;

        let res = gp.client().get(url).send().await;
        if let Err(e) = res {
            tracing::error!(%e);
            return Err(e.into());
        }

        let res = res.unwrap().json::<User>().await;
        match res {
            Ok(ref user) => tracing::info!("Retrieved user: {user:?}"),
            Err(ref e) => tracing::error!(%e),
        }

        Ok(res?)
    }
}
