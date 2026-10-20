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
    watched_teams: &[String],
) -> Vec<QueueItem> {
    let mut queue: Vec<QueueItem> = pull_requests
        .iter()
        .filter(|pr| pr.author.login != viewer)
        .filter_map(|pr| {
            let personal = pr
                .requests
                .iter()
                .find(|r| matches!(&r.reviewer, Reviewer::User(login) if login == viewer));
            if let Some(request) = personal {
                let tier = if pr.viewer_has_reviewed {
                    PriorityTier::PersonalReReview
                } else {
                    PriorityTier::PersonalFirstReview
                };
                return Some(item(pr, tier, request.requested_at, None));
            }

            let (team, request) = pr.requests.iter().find_map(|r| match &r.reviewer {
                Reviewer::Team(team) if watched_teams.contains(team) => Some((team, r)),
                _ => None,
            })?;
            let tier = if pr.viewer_has_reviewed {
                PriorityTier::TeamReReview
            } else {
                PriorityTier::TeamFirstReview
            };
            Some(item(pr, tier, request.requested_at, Some(team.clone())))
        })
        .collect();
    queue.sort_by_key(|i| (i.tier, i.pull_request.author.is_bot, i.requested_at));
    queue
}

fn item(
    pr: &PullRequest,
    tier: PriorityTier,
    requested_at: Timestamp,
    team: Option<String>,
) -> QueueItem {
    QueueItem {
        pull_request: pr.clone(),
        tier,
        requested_at,
        team,
    }
}
