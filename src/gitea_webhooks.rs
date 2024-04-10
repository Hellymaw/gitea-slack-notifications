use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Debug)]
pub struct OutgoingWebhook {
    pub email: String,
    pub title: String,
    pub body: String,
}
