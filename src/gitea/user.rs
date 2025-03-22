use reqwest::Client;
use serde::Deserialize;
use tracing::{self, instrument};
use url::Url;

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

    // TODO: Change to better err type
    #[instrument]
    pub async fn fetch_from_username(url: &mut Url, username: &str) -> Result<User, anyhow::Error> {
        // TODO change from this path formatting
        url.set_path(format!("api/v1/users/{}", username).as_str());

        let res = Client::new().get(url.as_str()).send().await;

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
