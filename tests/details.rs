use review_queue::details::description_lines;

fn lines(body: &str) -> Vec<String> {
    description_lines(body, 40, 20)
}

#[test]
fn html_comments_are_removed_even_across_lines() {
    let body = "Fixes the cache.\n<!-- Describe your change.\n     Link the issue. -->\nCloses #12. <!-- inline -->";

    assert_eq!(lines(body), vec!["Fixes the cache.", "Closes #12."]);
}

#[test]
fn images_are_removed() {
    let body = "Before:\n![screenshot](https://example.com/a.png)\nAfter: <img src=\"b.png\" width=\"300\"> done";

    assert_eq!(lines(body), vec!["Before:", "After: done"]);
}

#[test]
fn runs_of_blank_lines_become_one_and_blank_lines_at_the_ends_are_removed() {
    let body = "\n\n  \nFirst.\n\n\n\nSecond.   \n\n";

    assert_eq!(lines(body), vec!["First.", "", "Second."]);
}

#[test]
fn long_lines_wrap_at_word_boundaries_and_long_words_are_split() {
    let body = "The quick brown fox jumps over the lazy dog again.\nhttps://example.com/a/very/long/path/that/does/not/fit";

    assert_eq!(
        description_lines(body, 20, 20),
        vec![
            "The quick brown fox",
            "jumps over the lazy",
            "dog again.",
            "https://example.com/",
            "a/very/long/path/tha",
            "t/does/not/fit",
        ]
    );
}

#[test]
fn text_that_does_not_fit_is_cut_and_the_last_line_ends_with_an_ellipsis() {
    let body = "One.\nTwo.\nThree.\nFour.";

    assert_eq!(
        description_lines(body, 40, 3),
        vec!["One.", "Two.", "Three.…"]
    );
}

#[test]
fn an_empty_description_has_no_lines() {
    assert!(lines("").is_empty());
    assert!(lines("<!-- only a template -->\n\n").is_empty());
}

use review_queue::details::panel_layout;

#[test]
fn the_panel_is_below_the_list_at_24_rows_or_more_and_uses_40_percent_of_the_height() {
    assert_eq!(panel_layout(23), (23, None));
    assert_eq!(panel_layout(24), (12, Some(12)));
    assert_eq!(panel_layout(40), (24, Some(16)));
    assert_eq!(panel_layout(60), (36, Some(24)));
    assert_eq!(panel_layout(0), (0, None));
}

use jiff::{Timestamp, Zoned};
use review_queue::details::detail_fields;
use review_queue::queue::{Author, CiStatus, Details, Label, PriorityTier, PullRequest, QueueItem};

fn now() -> Zoned {
    "2026-10-21T10:00[Australia/Melbourne]".parse().unwrap()
}

fn local(s: &str) -> Timestamp {
    format!("{s}[Australia/Melbourne]")
        .parse::<Zoned>()
        .unwrap()
        .timestamp()
}

fn item() -> QueueItem {
    QueueItem {
        pull_request: PullRequest {
            repo: "acme/widgets".into(),
            number: 42,
            title: "Add sprockets".into(),
            author: Author {
                login: "alice".into(),
                is_bot: false,
                avatar_url: None,
            },
            is_draft: false,
            additions: 120,
            deletions: 30,
            requests: Vec::new(),
            viewer_has_reviewed: false,
            latest_mention_at: None,
            details: Details {
                head_ref: "alice/sprockets".into(),
                base_ref: "main".into(),
                changed_files: 7,
                labels: Vec::new(),
                ci: CiStatus::Passing,
                body: String::new(),
            },
        },
        tier: PriorityTier::PersonalFirstReview,
        requested_at: local("2026-10-18T10:00"),
        team: None,
    }
}

#[test]
fn the_fields_of_a_plain_request() {
    assert_eq!(
        detail_fields(&item(), None, &now()),
        vec![
            ("PR", "acme/widgets#42".to_string()),
            ("Author", "@alice".to_string()),
            ("Tier", "Personal first review".to_string()),
            ("Waiting", "3d".to_string()),
            ("Branch", "alice/sprockets → main".to_string()),
            ("Changes", "7 files · +120 −30".to_string()),
            ("CI", "✓ passing".to_string()),
        ]
    );
}

#[test]
fn optional_fields_show_when_they_apply() {
    let mut item = item();
    item.tier = PriorityTier::TeamReReview;
    item.team = Some("acme/core".into());
    item.pull_request.is_draft = true;
    item.pull_request.author.is_bot = true;
    item.pull_request.details.ci = CiStatus::Failing;
    item.pull_request.details.labels = vec![
        Label {
            name: "enhancement".into(),
            color: [0, 0, 0],
        },
        Label {
            name: "needs-docs".into(),
            color: [0, 0, 0],
        },
    ];

    let fields = detail_fields(&item, Some(local("2026-10-23T15:30")), &now());

    assert_eq!(
        fields,
        vec![
            ("PR", "acme/widgets#42 · draft".to_string()),
            ("Author", "@alice (bot)".to_string()),
            ("Tier", "Team re-review · @acme/core".to_string()),
            ("Waiting", "3d".to_string()),
            ("Snoozed", "until Fri 15:30".to_string()),
            ("Branch", "alice/sprockets → main".to_string()),
            ("Changes", "7 files · +120 −30".to_string()),
            ("CI", "✗ failing".to_string()),
            ("Labels", "enhancement, needs-docs".to_string()),
        ]
    );
}

#[test]
fn each_ci_status_has_a_symbol_and_a_word() {
    let ci = |status| {
        let mut item = item();
        item.pull_request.details.ci = status;
        detail_fields(&item, None, &now())
            .into_iter()
            .find(|(name, _)| *name == "CI")
            .unwrap()
            .1
    };

    assert_eq!(ci(CiStatus::Passing), "✓ passing");
    assert_eq!(ci(CiStatus::Failing), "✗ failing");
    assert_eq!(ci(CiStatus::Pending), "● pending");
    assert_eq!(ci(CiStatus::NoChecks), "– no checks");
}
