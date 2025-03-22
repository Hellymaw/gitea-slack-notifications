use serde::Deserialize;
use strum::Display;
use url::Url;

use crate::gitea::user::User;


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
pub struct Comment {
    pub body: String,
}

impl Comment {
    pub async fn parse_mentions(&self) -> Vec<String> {
        self
            .body
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                if line.starts_with(">") {
                    None
                } else {
                    Some(line)
                }
            })
            .flat_map(|x| x.split_whitespace())
            .filter_map(|x| {
                if x.starts_with("@") {
                    Some(x.trim_start_matches("@").to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub body: String,
    pub comments: u64,
    pub id: u64,
    pub user: User,
    pub title: String,
    #[serde(rename = "html_url")]
    pub url: Url,
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
    #[strum(serialize = "commented on")]
    Comment { content: String },
}

#[derive(Deserialize, Debug, Display)]
#[serde(rename_all = "snake_case", tag = "action")]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Opened,
    Closed,
    Reopened,
    Merged,
    Created { comment: Comment },
    Reviewed { review: Review },
    ReviewRequested { requested_reviewer: User },
}

#[derive(Deserialize, Debug)]
pub struct Webhook {
    #[serde(flatten)]
    pub action: Action,
    #[serde(alias = "issue")]
    pub pull_request: PullRequest,
    pub sender: User,
    pub repository: Repository,
}
