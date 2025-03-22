use slack_morphism::prelude::*;
use tracing::{self, instrument};

#[instrument]
pub async fn fetch_user_from_email(email: String) -> Result<SlackUser, anyhow::Error> {
    let client = SlackClient::new(
        SlackClientHyperConnector::new()?.with_rate_control(SlackApiRateControlConfig::new()),
    );
    let session = client.open_session(&super::TOKEN);

    let request = SlackApiUsersLookupByEmailRequest::new(EmailAddress::new(email));
    let user = session.users_lookup_by_email(&request).await?;

    tracing::info!("Retrieved user: {user:?}");
    Ok(user.user)
}
