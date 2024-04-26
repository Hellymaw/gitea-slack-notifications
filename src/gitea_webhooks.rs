use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use slack_morphism::prelude::*;
use strum::Display;

#[derive(Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub username: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    Open,
    Closed,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub body: String,
    pub comments: u64,
    pub id: u64,
    pub user: User,
    pub title: String,
    pub url: String,
    pub state: PullRequestState,
}

#[derive(Deserialize, Debug, Display)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
pub enum Review {
    #[serde(rename = "pull_request_review_approved")]
    Approved { content: String },
    #[serde(rename = "pull_request_review_rejected")]
    Rejected { content: String },
    #[serde(rename = "pull_request_review_comment")]
    Comment { content: String },
}

#[derive(Deserialize, Debug, Display)]
#[serde(rename_all = "snake_case", tag = "action")]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Opened,
    Closed,
    Reopened,
    Edited,
    Merged,
    Reviewed { review: Review },
    ReviewRequested { requested_reviewer: User },
}

#[derive(Deserialize, Debug)]
pub struct Webhook {
    #[serde(flatten)]
    pub action: Action,
    pub pull_request: PullRequest,
    pub sender: User,
    pub repository: Repository,
}

#[derive(Serialize, Debug)]
pub struct OutgoingWebhook {
    pub email: String,
    pub title: String,
    pub body: String,
}

pub struct MySlackMessage<'a> {
    pub webhook: &'a Webhook,
    pub slack_user: Option<SlackUser>,
}

const GITEA_ADDRESS: &str = "http://localhost:3000/api/v1";

impl Webhook {
    pub async fn deanonymise_emails(
        mut self,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        self.sender.email = Webhook::fetch_gitea_user_email(&self.sender).await?;

        self.pull_request.user.email =
            Webhook::fetch_gitea_user_email(&self.pull_request.user).await?;

        if let Action::ReviewRequested {
            ref mut requested_reviewer,
        } = self.action
        {
            requested_reviewer.email = Webhook::fetch_gitea_user_email(&requested_reviewer).await?;
        }

        Ok(self)
    }

    async fn fetch_gitea_user_email(
        user: &User,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let token = std::env::var("GITEA_API_TOKEN")?;
        let url = format!("{}/users/{}", GITEA_ADDRESS, user.username);

        let res = Client::new()
            .get(url)
            .header("Authorization", "token ".to_string() + &token.to_owned())
            .send()
            .await?
            .json::<User>()
            .await?;

        Ok(res.email)
    }

    pub async fn into_my_slack(&self) -> MySlackMessage {
        let email = match self.action {
            Action::ReviewRequested {
                ref requested_reviewer,
            } => Some(&requested_reviewer.email),
            Action::Reviewed { review: _ } => Some(&self.pull_request.user.email),
            _ => None,
        };

        let slack_user = if let Some(email) = email {
            fetch_slack_user_from_email(email).await.ok()
        } else {
            None
        };

        MySlackMessage {
            webhook: self,
            slack_user,
        }
    }
}

impl SlackMessageTemplate for MySlackMessage<'_> {
    fn render_template(&self) -> SlackMessageContent {
        match &self.webhook.action {
            Action::Opened => render_pr_opened(&self.webhook),
            Action::Reviewed { review } => render_reviewed(self, review),
            Action::ReviewRequested { requested_reviewer } => {
                render_review_requested(self, &requested_reviewer)
            }
            _ => render_basic_action(&self.webhook),
        }
    }
}

fn format_pull_request_url(pull_request: &PullRequest) -> String {
    format!("<{}|{}>", pull_request.url, pull_request.title)
}

fn render_basic_action(webhook: &Webhook) -> SlackMessageContent {
    SlackMessageContent::new().with_blocks(slack_blocks![some_into(
        SlackSectionBlock::new().with_text(md!(
            "{} was {}",
            format_pull_request_url(&webhook.pull_request),
            webhook.action
        ))
    )])
}

fn render_reviewed(slack_message: &MySlackMessage, review: &Review) -> SlackMessageContent {
    if let Some(user) = &slack_message.slack_user {
        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has {} your PR",
                user.id.to_slack_format(),
                slack_message.webhook.sender.username,
                review
            ))
        )])
    } else {
        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has {} your PR",
                slack_message.webhook.pull_request.user.username,
                slack_message.webhook.sender.username,
                review
            ))
        )])
    }
}

fn render_review_requested(slack_message: &MySlackMessage, reviewer: &User) -> SlackMessageContent {
    if let Some(user) = &slack_message.slack_user {
        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has requested you to review {}",
                user.id.to_slack_format(),
                slack_message.webhook.sender.username,
                format_pull_request_url(&slack_message.webhook.pull_request)
            ))
        )])
    } else {
        SlackMessageContent::new().with_blocks(slack_blocks![some_into(
            SlackSectionBlock::new().with_text(md!(
                "{}, {} has requested you to review {}",
                reviewer.username,
                slack_message.webhook.sender.username,
                format_pull_request_url(&slack_message.webhook.pull_request)
            ))
        )])
    }
}

fn render_pr_opened(webhook: &Webhook) -> SlackMessageContent {
    let repo_name = webhook
        .repository
        .full_name
        .split_once("/")
        .expect("Invalid full_name field!");

    SlackMessageContent::new().with_blocks(slack_blocks![
        some_into(SlackHeaderBlock::new(pt!(
            "{} | {}",
            repo_name.0,
            repo_name.1
        ))),
        some_into(SlackSectionBlock::new().with_text(md!(
            "Pull request {} opened by {}",
            format_pull_request_url(&webhook.pull_request),
            webhook.sender.username
        ))),
        some_into(SlackSectionBlock::new().with_text(md!(">{}", webhook.pull_request.body)))
    ])
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

async fn fetch_slack_user_from_email(
    email: &str,
) -> Result<SlackUser, Box<dyn std::error::Error + Send + Sync>> {
    let client = SlackClient::new(SlackClientHyperConnector::new()?);
    let token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_TOKEN")?.into();
    let token: SlackApiToken = SlackApiToken::new(token_value);
    let session = client.open_session(&token);

    let email = EmailAddress(email.to_string());
    println!("email: {:?}", email);

    let request = SlackApiUsersLookupByEmailRequest::new(email);
    let slack_user = session.users_lookup_by_email(&request).await;
    println!("users: {:?}", slack_user);

    let slack_user = slack_user?;

    Ok(slack_user.user)
}
