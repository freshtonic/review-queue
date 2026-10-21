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
    /// The latest time that a comment or review mentioned the user personally.
    pub latest_mention_at: Option<Timestamp>,
    pub details: Details,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Author {
    pub login: String,
    pub is_bot: bool,
    pub avatar_url: Option<String>,
}

/// The data for the detail panel and for the links to the pull request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Details {
    pub head_ref: String,
    pub base_ref: String,
    pub changed_files: u64,
    pub labels: Vec<Label>,
    pub ci: CiStatus,
    /// The description, as the author wrote it (Markdown).
    pub body: String,
    /// The last commit of the head branch.
    pub head_commit: String,
    /// The commit of the user's latest review, if the user submitted one.
    pub reviewed_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub color: [u8; 3],
}

/// The check rollup state of the last commit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CiStatus {
    Passing,
    Failing,
    Pending,
    #[default]
    NoChecks,
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

/// Repositories and authors that the user never wants in the Queue. The matches are
/// exact, without regard to case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exclusions {
    /// As "owner/name".
    pub repos: Vec<String>,
    /// As logins.
    pub authors: Vec<String>,
}

impl Exclusions {
    fn excludes(&self, pr: &PullRequest) -> bool {
        let matches =
            |list: &[String], value: &str| list.iter().any(|v| v.eq_ignore_ascii_case(value));
        matches(&self.repos, &pr.repo) || matches(&self.authors, &pr.author.login)
    }
}

pub fn build_queue(
    pull_requests: &[PullRequest],
    viewer: &str,
    watched_teams: &[String],
    exclusions: &Exclusions,
) -> Vec<QueueItem> {
    let mut queue: Vec<QueueItem> = pull_requests
        .iter()
        .filter(|pr| pr.author.login != viewer)
        .filter(|pr| !exclusions.excludes(pr))
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

/// The name of the tier in plain output.
pub fn tier_name(tier: PriorityTier) -> &'static str {
    match tier {
        PriorityTier::PersonalReReview => "personal-re-review",
        PriorityTier::PersonalFirstReview => "personal-first-review",
        PriorityTier::TeamReReview => "team-re-review",
        PriorityTier::TeamFirstReview => "team-first-review",
    }
}

/// The tier with this name (as `tier_name` gives it).
pub fn parse_tier(name: &str) -> Option<PriorityTier> {
    [
        PriorityTier::PersonalReReview,
        PriorityTier::PersonalFirstReview,
        PriorityTier::TeamReReview,
        PriorityTier::TeamFirstReview,
    ]
    .into_iter()
    .find(|&tier| tier_name(tier) == name)
}

/// The Queue as plain text for scripts, one line for each item, in Queue order. The
/// fields are separated by tabs: tier, `repo#number`, title, author, Wait Time, and the
/// Watched Team for a Team Request.
pub fn plain_lines(queue: &[QueueItem], now: Timestamp) -> String {
    queue
        .iter()
        .map(|item| {
            let pr = &item.pull_request;
            let mut fields = vec![
                tier_name(item.tier).to_string(),
                format!("{}#{}", pr.repo, pr.number),
                pr.title.replace(['\t', '\n'], " "),
                pr.author.login.clone(),
                crate::app::short_duration(item.requested_at, now),
            ];
            fields.extend(item.team.clone());
            fields.join("\t") + "\n"
        })
        .collect()
}

/// Whether a request is Fresh, Stale or Overdue: its Wait Time in working time, against
/// the user's thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lateness {
    Fresh,
    Stale,
    Overdue,
}

/// The working time after which a request is Stale, and after which it is Overdue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatenessThresholds {
    pub stale_after: jiff::SignedDuration,
    pub overdue_after: jiff::SignedDuration,
}

impl Default for LatenessThresholds {
    fn default() -> Self {
        Self {
            stale_after: jiff::SignedDuration::from_hours(4),
            overdue_after: jiff::SignedDuration::from_hours(8),
        }
    }
}

pub fn lateness(
    requested_at: Timestamp,
    now: &jiff::Zoned,
    working: &crate::snooze::WorkingTime,
    thresholds: &LatenessThresholds,
) -> Lateness {
    let waited = crate::snooze::working_duration(requested_at, now, working);
    if waited >= thresholds.overdue_after {
        Lateness::Overdue
    } else if waited >= thresholds.stale_after {
        Lateness::Stale
    } else {
        Lateness::Fresh
    }
}
