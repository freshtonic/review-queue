//! The Queue: all current Review Requests for the user, in priority order.
//!
//! This module is pure. It has no network or terminal dependencies.

use jiff::Timestamp;

/// An open pull request, as GitHub reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: Author,
    pub is_draft: bool,
    pub additions: u64,
    pub deletions: u64,
    pub requests: Vec<ReviewerRequest>,
    /// True when the user has submitted at least one review, in any state.
    pub viewer_has_reviewed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub login: String,
    pub is_bot: bool,
}

/// A requested reviewer on a pull request, with the time of its latest request event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerRequest {
    pub reviewer: Reviewer,
    pub requested_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reviewer {
    User(String),
    /// A team, as "org/team-slug".
    Team(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityTier {
    PersonalReReview,
    PersonalFirstReview,
    TeamReReview,
    TeamFirstReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub pull_request: PullRequest,
    pub tier: PriorityTier,
    /// The start of the Wait Time.
    pub requested_at: Timestamp,
    /// The Watched Team, for a Team Request.
    pub team: Option<String>,
}

pub fn build_queue(
    pull_requests: &[PullRequest],
    viewer: &str,
    _watched_teams: &[String],
) -> Vec<QueueItem> {
    pull_requests
        .iter()
        .filter_map(|pr| {
            let personal = pr
                .requests
                .iter()
                .find(|r| r.reviewer == Reviewer::User(viewer.to_string()))?;
            Some(QueueItem {
                pull_request: pr.clone(),
                tier: if pr.viewer_has_reviewed {
                    PriorityTier::PersonalReReview
                } else {
                    PriorityTier::PersonalFirstReview
                },
                requested_at: personal.requested_at,
                team: None,
            })
        })
        .collect()
}
