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
                avatar_url: Some("https://avatars.example/alice".into()),
            },
            is_draft: true,
            additions: 120,
            deletions: 30,
            requests: vec![ReviewerRequest {
                reviewer: Reviewer::User("me".into()),
                requested_at: "2026-10-01T09:00:00Z".parse().unwrap(),
            }],
            viewer_has_reviewed: false,
            latest_mention_at: None,
            details: Details {
                head_ref: "feature".into(),
                base_ref: "main".into(),
                changed_files: 1,
                head_commit: format!("c0ffee{:034}", 42),
                ..Details::default()
            },
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
            is_bot: true,
            avatar_url: Some("https://avatars.example/renovate".into()),
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

use review_queue::queue::{CiStatus, Details, Label};

#[test]
fn the_details_of_a_pull_request_are_parsed() {
    let pr = only_pull_request("details");

    assert_eq!(
        pr.author.avatar_url.as_deref(),
        Some("https://avatars.example/u/1?v=4")
    );
    assert_eq!(
        pr.details,
        Details {
            head_ref: "alice/sprockets".into(),
            base_ref: "main".into(),
            changed_files: 7,
            labels: vec![
                Label {
                    name: "enhancement".into(),
                    color: [0xa2, 0xee, 0xef],
                },
                Label {
                    name: "needs-docs".into(),
                    color: [0xd9, 0x3f, 0x0b],
                },
            ],
            ci: CiStatus::Failing,
            body: "Adds sprockets.\n\n<!-- template comment -->\nCloses #12.".into(),
            head_commit: format!("c0ffee{:034}", pr.number),
            reviewed_commit: None,
        }
    );
}

#[test]
fn each_check_rollup_state_maps_to_a_ci_status() {
    let cases = [
        (r#"{ "state": "SUCCESS" }"#, CiStatus::Passing),
        (r#"{ "state": "FAILURE" }"#, CiStatus::Failing),
        (r#"{ "state": "ERROR" }"#, CiStatus::Failing),
        (r#"{ "state": "PENDING" }"#, CiStatus::Pending),
        (r#"{ "state": "EXPECTED" }"#, CiStatus::Pending),
        ("null", CiStatus::NoChecks),
    ];
    for (rollup, expected) in cases {
        let body = fixture("details").replace(r#"{ "state": "FAILURE" }"#, rollup);
        let snapshot = parse_response(&body).unwrap();
        assert_eq!(snapshot.pull_requests[0].details.ci, expected, "{rollup}");
    }
    let no_commits = fixture("details").replace(
        r#"{ "nodes": [ { "commit": { "statusCheckRollup": { "state": "FAILURE" } } } ] }"#,
        r#"{ "nodes": [] }"#,
    );
    let snapshot = parse_response(&no_commits).unwrap();
    assert_eq!(snapshot.pull_requests[0].details.ci, CiStatus::NoChecks);
}

#[test]
fn a_re_review_has_the_commit_of_the_viewers_last_review_and_the_head_commit() {
    let pr = only_pull_request("re_review");

    assert!(pr.viewer_has_reviewed);
    assert_eq!(
        pr.details.reviewed_commit.as_deref(),
        Some("1111111111111111111111111111111111111111")
    );
    assert_eq!(
        pr.details.head_commit,
        "2222222222222222222222222222222222222222"
    );
}

#[test]
fn a_response_with_more_results_has_the_cursor_of_the_next_page() {
    let snapshot = parse_response(&fixture("next_page")).unwrap();
    assert_eq!(snapshot.next_page.as_deref(), Some("Y3Vyc29yOjEwMA=="));
    assert_eq!(snapshot.pull_requests.len(), 1);

    let last = parse_response(&fixture("personal_request")).unwrap();
    assert_eq!(last.next_page, None);
}
