//! GitHub's GraphQL API: the query for the Queue, and the conversion of its response
//! to domain data.

use graphql_client::{GraphQLQuery, Response};

use crate::queue::{Author, PullRequest, Reviewer, ReviewerRequest};

type DateTime = jiff::Timestamp;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/review_requests.graphql",
    response_derives = "Debug, Clone"
)]
pub struct ReviewRequests;

use review_requests as q;

/// The data from one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The login of the user.
    pub viewer: String,
    pub pull_requests: Vec<PullRequest>,
}

#[derive(Debug)]
pub struct ParseError(pub String);

/// Runs the query against github.com.
pub async fn fetch(client: &reqwest::Client, token: &str) -> Result<Snapshot, String> {
    let response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header(reqwest::header::USER_AGENT, "review-queue")
        .json(&ReviewRequests::build_query(q::Variables))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("GitHub returned {status}"));
    }
    parse_response(&body).map_err(|e| e.0)
}

pub fn parse_response(body: &str) -> Result<Snapshot, ParseError> {
    let response: Response<q::ResponseData> =
        serde_json::from_str(body).map_err(|e| ParseError(e.to_string()))?;
    let errors = response.errors.unwrap_or_default();
    if !errors.is_empty() {
        let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
        return Err(ParseError(messages.join("; ")));
    }
    let data = response
        .data
        .ok_or_else(|| ParseError("the response has no data".into()))?;
    let pull_requests = data
        .search
        .nodes
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter_map(|node| match node {
            q::ReviewRequestsSearchNodes::PullRequest(pr) => {
                Some(pull_request(pr, &data.viewer.login))
            }
            _ => None,
        })
        .collect();
    Ok(Snapshot {
        viewer: data.viewer.login,
        pull_requests,
    })
}

fn pull_request(pr: q::ReviewRequestsSearchNodesOnPullRequest, viewer: &str) -> PullRequest {
    use q::ReviewRequestsSearchNodesOnPullRequestTimelineItemsNodes as Node;
    let timeline: Vec<Node> = pr
        .timeline_items
        .nodes
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect();

    // The latest request event for each reviewer is the start of its Wait Time.
    let events: Vec<(Reviewer, jiff::Timestamp)> = timeline
        .iter()
        .filter_map(|node| match node {
            Node::ReviewRequestedEvent(e) => {
                Some((event_reviewer(e.requested_reviewer.clone()?)?, e.created_at))
            }
            _ => None,
        })
        .collect();

    // GitHub logins are not case-sensitive.
    let latest_mention_at = timeline
        .iter()
        .filter_map(|node| match node {
            Node::MentionedEvent(e) => e
                .actor
                .as_ref()
                .filter(|actor| actor.login.eq_ignore_ascii_case(viewer))
                .map(|_| e.created_at),
            _ => None,
        })
        .max();

    let requests = pr
        .review_requests
        .and_then(|r| r.nodes)
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter_map(|node| requested_reviewer(node.requested_reviewer?))
        .map(|reviewer| {
            let requested_at = events
                .iter()
                .filter(|(r, _)| *r == reviewer)
                .map(|(_, at)| *at)
                .max()
                .unwrap_or(pr.created_at);
            ReviewerRequest {
                reviewer,
                requested_at,
            }
        })
        .collect();

    let author = pr.author.map_or(
        Author {
            login: "ghost".into(),
            is_bot: false,
        },
        |a| Author {
            is_bot: matches!(a.on, q::ReviewRequestsSearchNodesOnPullRequestAuthorOn::Bot),
            login: a.login,
        },
    );

    PullRequest {
        repo: pr.repository.name_with_owner,
        number: pr.number as u64,
        title: pr.title,
        author,
        is_draft: pr.is_draft,
        additions: pr.additions as u64,
        deletions: pr.deletions as u64,
        requests,
        viewer_has_reviewed: pr.viewer_latest_review.is_some(),
        latest_mention_at,
    }
}

fn requested_reviewer(
    reviewer: q::ReviewRequestsSearchNodesOnPullRequestReviewRequestsNodesRequestedReviewer,
) -> Option<Reviewer> {
    use q::ReviewRequestsSearchNodesOnPullRequestReviewRequestsNodesRequestedReviewer as R;
    match reviewer {
        R::User(u) => Some(Reviewer::User(u.login)),
        R::Team(t) => Some(Reviewer::Team(t.combined_slug)),
        _ => None,
    }
}

fn event_reviewer(
    reviewer: q::ReviewRequestsSearchNodesOnPullRequestTimelineItemsNodesOnReviewRequestedEventRequestedReviewer,
) -> Option<Reviewer> {
    use q::ReviewRequestsSearchNodesOnPullRequestTimelineItemsNodesOnReviewRequestedEventRequestedReviewer as R;
    match reviewer {
        R::User(u) => Some(Reviewer::User(u.login)),
        R::Team(t) => Some(Reviewer::Team(t.combined_slug)),
        _ => None,
    }
}
