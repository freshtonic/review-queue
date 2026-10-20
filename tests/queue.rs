use jiff::Timestamp;
use review_queue::queue::{
    Author, PriorityTier, PullRequest, Reviewer, ReviewerRequest, build_queue,
};

const ME: &str = "me";

fn at(s: &str) -> Timestamp {
    s.parse().unwrap()
}

fn pr(number: u64) -> PullRequest {
    PullRequest {
        repo: "acme/widgets".into(),
        number,
        title: format!("PR {number}"),
        author: Author {
            login: "alice".into(),
            is_bot: false,
        },
        is_draft: false,
        additions: 10,
        deletions: 2,
        requests: Vec::new(),
        viewer_has_reviewed: false,
    }
}

fn requesting_me(mut pr: PullRequest, at_time: &str) -> PullRequest {
    pr.requests.push(ReviewerRequest {
        reviewer: Reviewer::User(ME.into()),
        requested_at: at(at_time),
    });
    pr
}

#[test]
fn a_pull_request_that_requests_only_me_is_a_personal_first_review() {
    let prs = [requesting_me(pr(1), "2026-10-01T09:00:00Z")];

    let queue = build_queue(&prs, ME, &[]);

    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].pull_request.number, 1);
    assert_eq!(queue[0].tier, PriorityTier::PersonalFirstReview);
    assert_eq!(queue[0].requested_at, at("2026-10-01T09:00:00Z"));
    assert_eq!(queue[0].team, None);
}
