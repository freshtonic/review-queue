use jiff::Timestamp;
use review_queue::notify::{Notice, NotifySettings, to_notify};
use review_queue::queue::{Author, PriorityTier, PullRequest, QueueItem};

fn at(s: &str) -> Timestamp {
    s.parse().unwrap()
}

fn item(number: u64, tier: PriorityTier, requested_at: &str) -> QueueItem {
    QueueItem {
        pull_request: PullRequest {
            repo: "acme/widgets".into(),
            number,
            title: format!("PR {number}"),
            author: Author {
                login: "alice".into(),
                is_bot: false,
                avatar_url: None,
            },
            is_draft: false,
            additions: 1,
            deletions: 1,
            requests: Vec::new(),
            viewer_has_reviewed: false,
            latest_mention_at: None,
            details: Default::default(),
        },
        tier,
        requested_at: at(requested_at),
        team: matches!(
            tier,
            PriorityTier::TeamReReview | PriorityTier::TeamFirstReview
        )
        .then(|| "acme/platform".to_string()),
    }
}

fn defaults() -> NotifySettings {
    NotifySettings::default()
}

const NOW: &str = "2026-10-21T10:00:00Z";

fn numbers(notices: &[Notice]) -> Vec<u64> {
    notices.iter().map(|n| n.number).collect()
}

#[test]
fn a_new_request_in_a_notifying_tier_gives_a_notice_with_the_tier_the_pr_and_the_author() {
    let before = vec![item(
        1,
        PriorityTier::PersonalFirstReview,
        "2026-10-20T09:00:00Z",
    )];
    let mut now = before.clone();
    now.push(item(
        2,
        PriorityTier::PersonalReReview,
        "2026-10-21T09:59:00Z",
    ));

    let notices = to_notify(Some(&before[..]), &now, &defaults(), None, at(NOW));

    assert_eq!(
        notices,
        vec![Notice {
            title: "Re-review requested".into(),
            subtitle: "acme/widgets#2".into(),
            body: "PR 2 · @alice".into(),
            repo: "acme/widgets".into(),
            number: 2,
        }]
    );
}

#[test]
fn the_default_tiers_are_personal_re_review_personal_first_review_and_team_re_review() {
    let now = vec![
        item(1, PriorityTier::PersonalReReview, NOW),
        item(2, PriorityTier::PersonalFirstReview, NOW),
        item(3, PriorityTier::TeamReReview, NOW),
        item(4, PriorityTier::TeamFirstReview, NOW),
    ];
    let notices = to_notify(Some(&[][..]), &now, &defaults(), None, at(NOW));
    assert_eq!(numbers(&notices), vec![1, 2, 3]);
    assert_eq!(notices[1].title, "Review requested");
    assert_eq!(
        notices[2].title,
        "Team re-review requested (@acme/platform)"
    );
}

#[test]
fn the_first_poll_gives_no_notices() {
    let now = vec![item(1, PriorityTier::PersonalReReview, NOW)];
    assert!(to_notify(None, &now, &defaults(), None, at(NOW)).is_empty());
}

#[test]
fn a_re_request_of_a_known_pull_request_gives_a_notice_and_a_known_request_does_not() {
    let before = [
        item(1, PriorityTier::PersonalFirstReview, "2026-10-20T09:00:00Z"),
        item(2, PriorityTier::PersonalFirstReview, "2026-10-20T09:00:00Z"),
    ];
    let now = vec![
        item(1, PriorityTier::PersonalReReview, "2026-10-21T09:00:00Z"),
        item(2, PriorityTier::PersonalFirstReview, "2026-10-20T09:00:00Z"),
    ];
    assert_eq!(
        numbers(&to_notify(
            Some(&before[..]),
            &now,
            &defaults(),
            None,
            at(NOW)
        )),
        vec![1]
    );
}

#[test]
fn nothing_is_notified_while_muted_or_when_notifications_are_off_or_for_other_tiers() {
    let now = vec![item(1, PriorityTier::PersonalReReview, NOW)];
    let muted = to_notify(
        Some(&[][..]),
        &now,
        &defaults(),
        Some(at("2026-10-21T11:00:00Z")),
        at(NOW),
    );
    assert!(muted.is_empty());
    // A mute that ended does not stop notices.
    let ended = to_notify(
        Some(&[][..]),
        &now,
        &defaults(),
        Some(at("2026-10-21T09:00:00Z")),
        at(NOW),
    );
    assert_eq!(numbers(&ended), vec![1]);

    let off = NotifySettings {
        enabled: false,
        ..defaults()
    };
    assert!(to_notify(Some(&[][..]), &now, &off, None, at(NOW)).is_empty());

    let team_only = NotifySettings {
        tiers: vec![PriorityTier::TeamFirstReview],
        ..defaults()
    };
    assert!(to_notify(Some(&[][..]), &now, &team_only, None, at(NOW)).is_empty());
}

#[test]
fn more_than_3_new_requests_give_3_notices_and_a_summary_that_selects_the_first_of_the_rest() {
    let now: Vec<QueueItem> = (1..=5)
        .map(|n| item(n, PriorityTier::PersonalFirstReview, NOW))
        .collect();
    let notices = to_notify(Some(&[][..]), &now, &defaults(), None, at(NOW));
    assert_eq!(numbers(&notices), vec![1, 2, 3, 4]);
    assert_eq!(
        notices[3],
        Notice {
            title: "2 more review requests".into(),
            subtitle: String::new(),
            body: "acme/widgets#4, acme/widgets#5".into(),
            repo: "acme/widgets".into(),
            number: 4,
        }
    );
}

#[test]
fn the_mute_is_saved_and_loaded_and_a_missing_or_bad_file_is_no_mute() {
    use review_queue::notify::{load_mute, save_mute};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state").join("mute.json");
    assert_eq!(load_mute(&path), None);

    let until = at("2026-10-21T12:00:00Z");
    save_mute(&path, Some(until)).unwrap();
    assert_eq!(load_mute(&path), Some(until));

    save_mute(&path, None).unwrap();
    assert_eq!(load_mute(&path), None);

    std::fs::write(&path, "not json").unwrap();
    assert_eq!(load_mute(&path), None);
}
