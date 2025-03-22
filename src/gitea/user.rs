use url::Url;
use reqwest::Client;
use serde::Deserialize;

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
    
    pub async fn fetch_from_username(
        url: &mut Url,
        username: &str,
    ) -> Result<User, anyhow::Error> {
        // TODO change from this path formatting
        url.set_path(format!("api/v1/users/{}", username).as_str());
    
        let res = Client::new()
            .get(url.as_str())
            .send()
            .await?
            .json::<User>()
            .await?;

        Ok(res)
    }
}

