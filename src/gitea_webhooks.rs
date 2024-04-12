use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;

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

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Review {
    #[serde(rename = "pull_request_review_approved")]
    Approved { content: String },
    #[serde(rename = "pull_request_review_rejected")]
    Rejected { content: String },
    #[serde(rename = "pull_request_review_comment")]
    Comment { content: String },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum Action {
    Opened,
    Closed,
    Reopened,
    Edited,
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

impl SlackMessageTemplate for Webhook {
    fn render_template(&self) -> SlackMessageContent {
        match self.action {
            Action::Opened => render_pr_opened(self),
            _ => todo!(),
        }
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
            "Pull request <{}|{}> opened by {}",
            webhook.pull_request.url,
            webhook.pull_request.title,
            webhook.sender.username
        ))),
        some_into(SlackSectionBlock::new().with_text(md!(">{}", webhook.pull_request.body)))
    ])
}
