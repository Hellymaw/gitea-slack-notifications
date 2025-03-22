use serde::Deserialize;
use std::iter;
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
    pub fn parse_mentions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(
            self.body
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
                        Some(x.trim_start_matches("@"))
                    } else {
                        None
                    }
                }),
        )
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

impl Webhook {
    pub fn usernames_to_mention(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self.action {
            Action::ReviewRequested {
                ref requested_reviewer,
            } => Box::new(iter::once(requested_reviewer.username())),
            Action::Reviewed { review: _ } => {
                Box::new(iter::once(self.pull_request.user.username()))
            }
            Action::Created { ref comment } => comment.parse_mentions(),
            _ => Box::new(iter::empty()),
        }
    }
}
