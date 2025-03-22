use serde::Serialize;
use slack_morphism::prelude::*;
use sqlx::PgPool;

use crate::gitea::webhook::*;
use crate::gitea::{self, GiteaResourcePool};
use crate::user_lookup::CachedUser;

#[derive(Serialize, Debug)]
pub struct OutgoingWebhook {
    pub email: String,
    pub title: String,
    pub body: String,
}

pub struct MySlackMessage {
    pub webhook: Webhook,
    pub slack_user: Vec<SlackUserId>,
}

impl MySlackMessage {
    pub async fn from_gitea_webhook(
        webhook: Webhook,
        db: &PgPool,
        gp: &GiteaResourcePool,
    ) -> Result<Option<Self>, anyhow::Error> {
        let usernames = webhook
            .usernames_to_mention()
            .map(|x| CachedUser::fetch(db, gp, webhook.pull_request.url.clone(), x));

        let mut users: Vec<SlackUserId> = Vec::new();
        for username in futures::future::join_all(usernames).await {
            users.push(username?.slack_uid().to_owned());
        }

        // If the webhook is for a comment but no-one was mentioned, there's no reason to send a message
        if let Action::Created { comment: _ } = webhook.action {
            if users.len() == 0 {
                return Ok(None);
            }
        }

        Ok(Some(Self {
            webhook,
            slack_user: users,
        }))
    }

    pub async fn post(&self, parent: &Option<SlackTs>) -> Result<SlackTs, anyhow::Error> {
        // TODO remove this
        let client = SlackClient::new(SlackClientHyperConnector::new()?);
        let token_value: SlackApiTokenValue = config_env_var("SLACK_API_TOKEN")?.into();
        let token = SlackApiToken::new(token_value);
        let session = client.open_session(&token);

        // TODO remove this, or lookup
        let channel = config_env_var("SLACK_CHANNEL")?;

        let message = self.render_template();
        let post_chat_req = if let Some(thread_ts) = parent {
            SlackApiChatPostMessageRequest::new(channel.into(), message)
                .with_thread_ts(thread_ts.clone())
        } else {
            SlackApiChatPostMessageRequest::new(channel.into(), message)
        };

        let post_chat_resp = session.chat_post_message(&post_chat_req).await?;

        Ok(post_chat_resp.ts)
    }

    fn render_comment(&self) -> SlackMessageContent {
        let mentions = self
            .slack_user
            .iter()
            .map(|x| x.to_slack_format())
            .collect::<Vec<String>>()
            .join(" ");

        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new()
                .with_text(md!("{}, you were mentioned in a comment", mentions))
        )])
    }

    fn render_reviewed(&self, review: &Review) -> SlackMessageContent {
        let user = if let Some(user) = self.slack_user.first() {
            user.to_slack_format()
        } else {
            self.webhook.pull_request.user.username().to_string()
        };

        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has {} your PR",
                user,
                self.webhook.sender.username(),
                review
            ))
        )])
    }

    fn render_review_requested(&self, reviewer: &gitea::user::User) -> SlackMessageContent {
        let user = if let Some(user) = self.slack_user.first() {
            user.to_slack_format()
        } else {
            reviewer.username().to_string()
        };

        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has requested you to review {}",
                user,
                self.webhook.sender.username(),
                format_pull_request_url(&self.webhook.pull_request)
            ))
        )])
    }

    fn render_pr_opened(&self) -> SlackMessageContent {
        let repo_name = self
            .webhook
            .repository
            .full_name
            .split_once("/")
            .expect("Invalid full_name field!");

        let body = self
            .webhook
            .pull_request
            .body
            .split_inclusive("\n")
            .map(|line| ">".to_string() + line)
            .collect::<Vec<String>>()
            .join("");

        SlackMessageContent::new().with_blocks(slack_blocks![
            some_into(SlackHeaderBlock::new(pt!(
                "{} | {}",
                repo_name.0,
                repo_name.1
            ))),
            some_into(SlackSectionBlock::new().with_text(md!(
                "Pull request {} opened by {}",
                format_pull_request_url(&self.webhook.pull_request),
                self.webhook.sender.username()
            ))),
            some_into(SlackSectionBlock::new().with_text(md!("{}", body)))
        ])
    }

    fn render_basic_action(&self) -> SlackMessageContent {
        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{} was {}",
                format_pull_request_url(&self.webhook.pull_request),
                self.webhook.action
            ))
        )])
    }
}

impl SlackMessageTemplate for MySlackMessage {
    fn render_template(&self) -> SlackMessageContent {
        match &self.webhook.action {
            Action::Opened => self.render_pr_opened(),
            Action::Reviewed { review } => self.render_reviewed(review),
            Action::ReviewRequested { requested_reviewer } => {
                self.render_review_requested(&requested_reviewer)
            }
            Action::Created { comment: _ } => self.render_comment(),
            _ => self.render_basic_action(),
        }
    }
}

fn format_pull_request_url(pull_request: &PullRequest) -> String {
    format!("<{}|{}>", pull_request.url, pull_request.title)
}

// TODO remove this
fn config_env_var(name: &str) -> Result<String, anyhow::Error> {
    Ok(std::env::var(name)?)
}
