pub mod user;
pub mod webhook;

#[derive(Debug)]
pub struct GiteaResourcePool {
    client: reqwest::Client,
}

impl GiteaResourcePool {
    pub fn new() -> Result<Self, anyhow::Error> {
        use std::time::Duration;

        let client = reqwest::ClientBuilder::new()
            .read_timeout(Duration::from_secs(90))
            .pool_idle_timeout(Some(Duration::from_secs(240)))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .build()?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
