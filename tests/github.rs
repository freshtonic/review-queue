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
