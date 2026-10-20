use review_queue::github::parse_response;
use review_queue::queue::{Author, PullRequest, Reviewer, ReviewerRequest};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}.json")).unwrap()
}

#[test]
fn a_personal_request_becomes_a_pull_request_with_the_time_of_its_request_event() {
    let snapshot = parse_response(&fixture("personal_request")).unwrap();

    assert_eq!(snapshot.viewer, "me");
    assert_eq!(
        snapshot.pull_requests,
        vec![PullRequest {
            repo: "acme/widgets".into(),
            number: 42,
            title: "Add sprockets".into(),
            author: Author {
                login: "alice".into(),
                is_bot: false,
            },
            is_draft: true,
            additions: 120,
            deletions: 30,
            requests: vec![ReviewerRequest {
                reviewer: Reviewer::User("me".into()),
                requested_at: "2026-10-01T09:00:00Z".parse().unwrap(),
            }],
            viewer_has_reviewed: false,
        }]
    );
}

fn only_pull_request(name: &str) -> PullRequest {
    let snapshot = parse_response(&fixture(name)).unwrap();
    assert_eq!(
        snapshot.pull_requests.len(),
        1,
        "results that are not pull requests are skipped"
    );
    snapshot.pull_requests.into_iter().next().unwrap()
}

#[test]
fn a_bot_author_is_marked_as_a_bot() {
    let pr = only_pull_request("mixed");

    assert_eq!(
        pr.author,
        Author {
            login: "renovate".into(),
            is_bot: true
        }
    );
}

#[test]
fn a_prior_review_in_any_state_means_the_viewer_has_reviewed() {
    let pr = only_pull_request("mixed");

    assert!(pr.viewer_has_reviewed);
}

#[test]
fn each_user_and_team_request_has_the_time_of_its_latest_request_event() {
    let pr = only_pull_request("mixed");

    assert_eq!(
        pr.requests,
        vec![
            ReviewerRequest {
                reviewer: Reviewer::Team("acme/core".into()),
                requested_at: "2026-10-02T09:00:00Z".parse().unwrap(),
            },
            ReviewerRequest {
                reviewer: Reviewer::User("me".into()),
                requested_at: "2026-10-04T09:00:00Z".parse().unwrap(),
            },
        ]
    );
}

#[test]
fn a_response_with_errors_and_no_data_is_an_error_that_includes_the_message() {
    let error = parse_response(&fixture("errors")).unwrap_err();

    assert!(error.0.contains("API rate limit exceeded"), "{}", error.0);
}
