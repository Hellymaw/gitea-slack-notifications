use slack_morphism::prelude::*;
use std::sync::LazyLock;

pub mod message;
pub mod user;

static TOKEN: LazyLock<SlackApiToken> = LazyLock::new(|| {
    // NOTE: Really need a SlackTeamId here...
    let raw_value = std::env::var("SLACK_API_TOKEN").unwrap().into();
    SlackApiToken::new(raw_value)
});
