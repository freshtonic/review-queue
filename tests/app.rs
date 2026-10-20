use jiff::{Timestamp, Zoned};
use review_queue::app::{Action, App, Effect, Event};
use review_queue::queue::{Author, PriorityTier, PullRequest, QueueItem};

fn at(s: &str) -> Timestamp {
    s.parse().unwrap()
}

fn item(number: u64) -> QueueItem {
    QueueItem {
        pull_request: PullRequest {
            repo: "acme/widgets".into(),
            number,
            title: format!("PR {number}"),
            author: Author {
                login: "alice".into(),
                is_bot: false,
            },
            is_draft: false,
            additions: 1,
            deletions: 1,
            requests: Vec::new(),
            viewer_has_reviewed: false,
            latest_mention_at: None,
        },
        tier: PriorityTier::PersonalFirstReview,
        requested_at: at("2026-10-01T09:00:00Z"),
        team: None,
    }
}

/// The time of the actions in these tests: Wednesday 21 October 2026, 10:00 in Melbourne.
fn now() -> Zoned {
    "2026-10-21T10:00[Australia/Melbourne]".parse().unwrap()
}

fn app_with(numbers: &[u64]) -> App {
    let mut app = App::default();
    app.handle(
        Event::PollSucceeded {
            queue: numbers.iter().map(|&n| item(n)).collect(),
            at: at("2026-10-20T09:00:00Z"),
        },
        &now(),
    );
    app
}

fn press(app: &mut App, action: Action) -> Effect {
    app.handle(Event::Action(action), &now())
}

fn selected_number(app: &App) -> Option<u64> {
    app.selected_item().map(|i| i.pull_request.number)
}

#[test]
fn down_and_up_move_the_selection_and_stop_at_the_ends_of_the_queue() {
    let mut app = app_with(&[1, 2, 3]);
    assert_eq!(selected_number(&app), Some(1));

    press(&mut app, Action::Up);
    assert_eq!(selected_number(&app), Some(1));

    press(&mut app, Action::Down);
    press(&mut app, Action::Down);
    press(&mut app, Action::Down);
    assert_eq!(selected_number(&app), Some(3));

    press(&mut app, Action::Up);
    assert_eq!(selected_number(&app), Some(2));
}

#[test]
fn open_gives_the_url_of_the_selected_pull_request() {
    let mut app = app_with(&[1, 2]);
    press(&mut app, Action::Down);

    let effect = press(&mut app, Action::Open);

    assert_eq!(
        effect,
        Effect::OpenUrl("https://github.com/acme/widgets/pull/2".into())
    );
}

#[test]
fn open_on_an_empty_queue_does_nothing() {
    let mut app = app_with(&[]);

    assert_eq!(press(&mut app, Action::Open), Effect::None);
}

#[test]
fn refresh_and_quit_ask_the_shell_to_refresh_and_quit() {
    let mut app = app_with(&[1]);

    assert_eq!(press(&mut app, Action::Refresh), Effect::Refresh);
    assert_eq!(press(&mut app, Action::Quit), Effect::Quit);
}

fn poll(app: &mut App, numbers: &[u64]) {
    app.handle(
        Event::PollSucceeded {
            queue: numbers.iter().map(|&n| item(n)).collect(),
            at: at("2026-10-20T09:01:00Z"),
        },
        &now(),
    );
}

#[test]
fn after_a_poll_the_selection_stays_on_the_same_pull_request() {
    let mut app = app_with(&[1, 2, 3]);
    press(&mut app, Action::Down);

    poll(&mut app, &[4, 3, 2, 1]);

    assert_eq!(selected_number(&app), Some(2));
}

#[test]
fn when_the_selected_pull_request_leaves_the_queue_the_selection_stays_in_the_queue() {
    let mut app = app_with(&[1, 2, 3]);
    press(&mut app, Action::Down);
    press(&mut app, Action::Down);

    poll(&mut app, &[1, 2]);

    assert_eq!(selected_number(&app), Some(2));
}

#[test]
fn before_the_first_poll_the_status_says_loading() {
    let app = App::default();

    assert_eq!(app.status(at("2026-10-20T09:00:00Z")), "loading…");
}

#[test]
fn the_status_shows_the_time_since_the_last_good_poll_and_the_count() {
    let app = app_with(&[1, 2, 3]);

    assert_eq!(
        app.status(at("2026-10-20T09:00:12Z")),
        "updated 12s ago · 3 pending"
    );
    assert_eq!(
        app.status(at("2026-10-20T09:05:00Z")),
        "updated 5m ago · 3 pending"
    );
}

#[test]
fn a_failed_poll_keeps_the_queue_and_shows_the_error_until_the_next_good_poll() {
    let mut app = app_with(&[1, 2]);

    app.handle(
        Event::PollFailed {
            error: "connection refused".into(),
        },
        &now(),
    );

    assert_eq!(app.queue.len(), 2);
    assert_eq!(
        app.status(at("2026-10-20T09:02:00Z")),
        "updated 2m ago · 2 pending · error: connection refused"
    );

    poll(&mut app, &[1]);
    assert_eq!(
        app.status(at("2026-10-20T09:01:00Z")),
        "updated 0s ago · 1 pending"
    );
}

#[test]
fn toggle_help_shows_and_closes_the_help_page() {
    let mut app = app_with(&[1]);
    assert!(!app.show_help);

    press(&mut app, Action::ToggleHelp);
    assert!(app.show_help);

    press(&mut app, Action::ToggleHelp);
    assert!(!app.show_help);
}

#[test]
fn while_help_is_open_movement_and_open_do_nothing_but_quit_still_quits() {
    let mut app = app_with(&[1, 2]);
    press(&mut app, Action::ToggleHelp);

    press(&mut app, Action::Down);
    assert_eq!(press(&mut app, Action::Open), Effect::None);
    assert_eq!(selected_number(&app), Some(1));

    assert_eq!(press(&mut app, Action::Quit), Effect::Quit);
}
