use slack_morphism::prelude::*;


pub async fn fetch_user_from_email(email: String) -> Result<SlackUser, anyhow::Error> {
    // TODO: Shift this management elsewhere
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let token_value: SlackApiTokenValue = config_env_var("SLACK_API_TOKEN")?.into();
    let token = SlackApiToken::new(token_value);
    let session = client.open_session(&token);

    let request = SlackApiUsersLookupByEmailRequest::new(EmailAddress::new(email));
    let slack_user = session.users_lookup_by_email(&request).await?;

    Ok(slack_user.user)
}

// TODO: Remove
fn config_env_var(name: &str) -> Result<String, anyhow::Error> {
    Ok(std::env::var(name)?)
}
