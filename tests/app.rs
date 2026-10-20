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

use review_queue::snooze::Snooze;

fn local(s: &str) -> Timestamp {
    format!("{s}[Australia/Melbourne]")
        .parse::<Zoned>()
        .unwrap()
        .timestamp()
}

fn snooze(number: u64, until: Timestamp) -> Snooze {
    Snooze {
        repo: "acme/widgets".into(),
        number,
        started_at: now().timestamp(),
        until,
    }
}

fn visible_numbers(app: &App) -> Vec<u64> {
    app.queue.iter().map(|i| i.pull_request.number).collect()
}

#[test]
fn dismiss_snoozes_the_selected_request_until_the_same_time_on_the_next_working_day() {
    let mut app = app_with(&[1, 2]);

    let effect = press(&mut app, Action::Dismiss);

    assert_eq!(
        effect,
        Effect::SaveSnoozes(vec![snooze(1, local("2026-10-22T10:00"))])
    );
    assert_eq!(visible_numbers(&app), vec![2]);
    assert_eq!(selected_number(&app), Some(2));
    assert_eq!(
        app.status(at("2026-10-20T09:01:00Z")),
        "updated 1m ago · 1 pending · 1 snoozed"
    );
}

use review_queue::snooze::WorkingTime;

fn poll_now(app: &mut App, numbers: &[u64]) -> Effect {
    app.handle(
        Event::PollSucceeded {
            queue: numbers.iter().map(|&n| item(n)).collect(),
            at: now().timestamp(),
        },
        &now(),
    )
}

#[test]
fn stored_snoozes_hide_their_requests_and_ended_snoozes_are_dropped_and_saved() {
    let active = snooze(1, local("2026-10-22T10:00"));
    let ended = snooze(2, local("2026-10-21T09:00"));
    let pull_request_gone = snooze(9, local("2026-10-22T10:00"));
    let mut app = App::new(
        WorkingTime::default(),
        vec![active.clone(), ended, pull_request_gone],
    );

    let effect = poll_now(&mut app, &[1, 2, 3]);

    assert_eq!(visible_numbers(&app), vec![2, 3]);
    assert_eq!(effect, Effect::SaveSnoozes(vec![active]));
}

#[test]
fn a_poll_that_changes_no_snooze_saves_nothing() {
    let active = snooze(1, local("2026-10-22T10:00"));
    let mut app = App::new(WorkingTime::default(), vec![active]);

    assert_eq!(poll_now(&mut app, &[1, 2]), Effect::None);
}

#[test]
fn snooze_opens_a_picker_with_numbered_choices_and_their_end_times() {
    let mut app = app_with(&[1]);

    assert_eq!(press(&mut app, Action::Snooze), Effect::None);

    let picker = app.picker.as_ref().expect("the picker is open");
    let choices: Vec<(&str, Option<Timestamp>)> = picker
        .choices
        .iter()
        .map(|c| (c.label.as_str(), c.until))
        .collect();
    assert_eq!(
        choices,
        vec![
            ("1 hour", Some(local("2026-10-21T11:00"))),
            ("3 hours", Some(local("2026-10-21T13:00"))),
            ("Next morning", Some(local("2026-10-22T09:00"))),
            ("Before COB Wed", Some(local("2026-10-21T15:30"))),
            ("Before COB Thu", Some(local("2026-10-22T15:30"))),
            ("Before COB Fri", Some(local("2026-10-23T15:30"))),
            ("Before COB Mon", Some(local("2026-10-26T15:30"))),
            ("Before COB Tue", Some(local("2026-10-27T15:30"))),
            ("Custom…", None),
        ]
    );
}

#[test]
fn choosing_a_numbered_choice_snoozes_the_selected_request_and_closes_the_picker() {
    let mut app = app_with(&[1, 2]);
    press(&mut app, Action::Snooze);

    let effect = press(&mut app, Action::Choose(2));

    assert_eq!(
        effect,
        Effect::SaveSnoozes(vec![snooze(1, local("2026-10-21T13:00"))])
    );
    assert!(app.picker.is_none());
    assert_eq!(visible_numbers(&app), vec![2]);
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, Action::Type(c));
    }
}

#[test]
fn custom_asks_for_a_duration_and_snoozes_for_it() {
    let mut app = app_with(&[1]);
    press(&mut app, Action::Snooze);
    press(&mut app, Action::Choose(9));
    assert_eq!(app.picker.as_ref().unwrap().input.as_deref(), Some(""));

    type_text(&mut app, "45mm");
    press(&mut app, Action::Backspace);
    let effect = press(&mut app, Action::Confirm);

    assert_eq!(
        effect,
        Effect::SaveSnoozes(vec![snooze(1, local("2026-10-21T10:45"))])
    );
    assert!(app.picker.is_none());
}

#[test]
fn a_custom_duration_that_is_not_valid_keeps_the_picker_open_with_an_error() {
    let mut app = app_with(&[1]);
    press(&mut app, Action::Snooze);
    press(&mut app, Action::Choose(9));
    type_text(&mut app, "soon");

    assert_eq!(press(&mut app, Action::Confirm), Effect::None);

    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.input.as_deref(), Some("soon"));
    assert!(picker.error.is_some());
    assert_eq!(visible_numbers(&app), vec![1]);
}

#[test]
fn cancel_closes_the_picker_and_snoozes_nothing() {
    let mut app = app_with(&[1, 2]);
    press(&mut app, Action::Snooze);

    assert_eq!(press(&mut app, Action::Cancel), Effect::None);

    assert!(app.picker.is_none());
    assert_eq!(visible_numbers(&app), vec![1, 2]);
}

#[test]
fn while_the_picker_is_open_movement_open_and_dismiss_do_nothing() {
    let mut app = app_with(&[1, 2]);
    press(&mut app, Action::Snooze);

    press(&mut app, Action::Down);
    assert_eq!(press(&mut app, Action::Open), Effect::None);
    assert_eq!(press(&mut app, Action::Dismiss), Effect::None);

    assert_eq!(selected_number(&app), Some(1));
    assert_eq!(visible_numbers(&app), vec![1, 2]);
}

fn snoozed_numbers(app: &App) -> Vec<u64> {
    app.snoozed
        .iter()
        .map(|s| s.item.pull_request.number)
        .collect()
}

#[test]
fn the_snoozed_view_selects_among_the_snoozed_requests() {
    let mut app = App::new(
        WorkingTime::default(),
        vec![
            snooze(1, local("2026-10-22T10:00")),
            snooze(3, local("2026-10-23T10:00")),
        ],
    );
    poll_now(&mut app, &[1, 2, 3]);

    press(&mut app, Action::ToggleSnoozedView);
    assert!(app.show_snoozed);
    assert_eq!(selected_number(&app), Some(1));
    press(&mut app, Action::Down);
    assert_eq!(selected_number(&app), Some(3));

    press(&mut app, Action::ToggleSnoozedView);
    assert!(!app.show_snoozed);
    assert_eq!(selected_number(&app), Some(2));
}

#[test]
fn unsnooze_in_the_snoozed_view_ends_the_snooze_of_the_selected_request() {
    let mut app = App::new(
        WorkingTime::default(),
        vec![
            snooze(1, local("2026-10-22T10:00")),
            snooze(3, local("2026-10-23T10:00")),
        ],
    );
    poll_now(&mut app, &[1, 2, 3]);
    press(&mut app, Action::ToggleSnoozedView);

    let effect = press(&mut app, Action::Unsnooze);

    assert_eq!(
        effect,
        Effect::SaveSnoozes(vec![snooze(3, local("2026-10-23T10:00"))])
    );
    assert_eq!(snoozed_numbers(&app), vec![3]);
    assert_eq!(visible_numbers(&app), vec![1, 2]);
    assert_eq!(selected_number(&app), Some(3));
}

#[test]
fn snooze_in_the_snoozed_view_changes_the_end_time() {
    let mut app = App::new(
        WorkingTime::default(),
        vec![snooze(1, local("2026-10-22T10:00"))],
    );
    poll_now(&mut app, &[1, 2]);
    press(&mut app, Action::ToggleSnoozedView);

    press(&mut app, Action::Snooze);
    let effect = press(&mut app, Action::Choose(1));

    assert_eq!(
        effect,
        Effect::SaveSnoozes(vec![snooze(1, local("2026-10-21T11:00"))])
    );
    assert_eq!(snoozed_numbers(&app), vec![1]);
}

#[test]
fn unsnooze_in_the_main_view_does_nothing() {
    let mut app = App::new(
        WorkingTime::default(),
        vec![snooze(1, local("2026-10-22T10:00"))],
    );
    poll_now(&mut app, &[1, 2]);

    assert_eq!(press(&mut app, Action::Unsnooze), Effect::None);
    assert_eq!(snoozed_numbers(&app), vec![1]);
}

#[test]
fn the_snooze_end_of_the_selected_request_is_known_only_in_the_snoozed_view() {
    let until = local("2026-10-22T10:00");
    let mut app = App::new(WorkingTime::default(), vec![snooze(1, until)]);
    poll_now(&mut app, &[1, 2]);

    assert_eq!(app.selected_snoozed_until(), None);
    press(&mut app, Action::ToggleSnoozedView);
    assert_eq!(app.selected_snoozed_until(), Some(until));
}

#[test]
fn with_the_panel_focused_up_and_down_scroll_the_description_not_the_selection() {
    let mut app = app_with(&[1, 2]);
    app.after_draw(true, 30);

    press(&mut app, Action::ToggleFocus);
    assert!(app.panel_focused);
    press(&mut app, Action::Down);
    press(&mut app, Action::Down);
    press(&mut app, Action::Up);
    assert_eq!(app.description_scroll, 1);
    assert_eq!(selected_number(&app), Some(1));

    press(&mut app, Action::ToggleFocus);
    assert!(!app.panel_focused);
    press(&mut app, Action::Down);
    assert_eq!(selected_number(&app), Some(2));
}

#[test]
fn the_scroll_stays_between_the_top_and_the_end_of_the_description() {
    let mut app = app_with(&[1]);
    app.after_draw(true, 12);
    press(&mut app, Action::ToggleFocus);

    press(&mut app, Action::Up);
    assert_eq!(app.description_scroll, 0);

    press(&mut app, Action::PageDown);
    assert_eq!(app.description_scroll, 10);
    press(&mut app, Action::PageDown);
    app.after_draw(true, 12);
    assert_eq!(app.description_scroll, 12);
    press(&mut app, Action::PageUp);
    assert_eq!(app.description_scroll, 2);
}

#[test]
fn a_new_selection_starts_the_description_at_the_top() {
    let mut app = app_with(&[1, 2]);
    app.after_draw(true, 30);
    press(&mut app, Action::ToggleFocus);
    press(&mut app, Action::PageDown);

    press(&mut app, Action::ToggleFocus);
    press(&mut app, Action::Down);

    assert_eq!(app.description_scroll, 0);
}

#[test]
fn when_the_panel_is_hidden_the_focus_goes_back_to_the_list() {
    let mut app = app_with(&[1, 2]);
    app.after_draw(true, 30);
    press(&mut app, Action::ToggleFocus);

    app.after_draw(false, 0);

    assert!(!app.panel_focused);
    press(&mut app, Action::Down);
    assert_eq!(selected_number(&app), Some(2));
}

fn arrivals(app: &App, seconds: f64) -> Vec<(u64, f32)> {
    let mut found: Vec<(u64, f32)> = app
        .arrival_progress(later(seconds).timestamp())
        .into_iter()
        .map(|((_, number), progress)| (number, progress))
        .collect();
    found.sort_by_key(|(number, _)| *number);
    found
}

fn later(seconds: f64) -> Zoned {
    now()
        .checked_add(jiff::SignedDuration::from_secs_f64(seconds))
        .unwrap()
}

#[test]
fn a_new_pull_request_at_a_poll_is_an_arrival_that_finishes_after_one_second() {
    let mut app = App::default();
    poll_now(&mut app, &[1]);

    poll_now(&mut app, &[1, 2, 3]);

    assert_eq!(arrivals(&app, 0.0), vec![(2, 0.0), (3, 0.0)]);
    let halfway = arrivals(&app, 0.5);
    assert!((halfway[0].1 - 0.5).abs() < 1e-3, "{halfway:?}");
    assert!(arrivals(&app, 1.0).is_empty());
}

#[test]
fn the_first_poll_has_no_arrivals() {
    let mut app = App::default();

    poll_now(&mut app, &[1, 2]);

    assert!(arrivals(&app, 0.0).is_empty());
}

#[test]
fn a_pull_request_back_from_a_snooze_is_an_arrival() {
    // The snooze ends 30 seconds after the first poll.
    let ends = later(30.0).timestamp();
    let mut app = App::new(WorkingTime::default(), vec![snooze(1, ends)]);
    poll_now(&mut app, &[1, 2]);
    assert_eq!(visible_numbers(&app), vec![2]);

    app.handle(
        Event::PollSucceeded {
            queue: [1, 2].iter().map(|&n| item(n)).collect(),
            at: later(60.0).timestamp(),
        },
        &later(60.0),
    );

    assert_eq!(visible_numbers(&app), vec![1, 2]);
    assert_eq!(arrivals(&app, 60.0), vec![(1, 0.0)]);
}

#[test]
fn a_pull_request_that_was_already_visible_is_not_an_arrival() {
    let mut app = App::default();
    poll_now(&mut app, &[1, 2]);

    poll_now(&mut app, &[2, 1]);

    assert!(arrivals(&app, 0.0).is_empty());
}

#[test]
fn the_attempt_key_shows_and_closes_the_attempt_image_over_a_full_queue() {
    let mut app = app_with(&[1, 2]);

    press(&mut app, Action::ToggleAttempt);
    assert!(app.show_attempt);
    press(&mut app, Action::Down);
    assert_eq!(selected_number(&app), Some(1));

    press(&mut app, Action::ToggleAttempt);
    assert!(!app.show_attempt);
}

use review_queue::app::arrival_color;

#[test]
fn the_arrival_color_fades_from_the_highlight_to_the_background_fast_then_slow() {
    let (highlight, background) = ([255, 200, 0], [0, 0, 0]);

    assert_eq!(arrival_color(0.0, highlight, background), [255, 200, 0]);
    assert_eq!(arrival_color(0.5, highlight, background), [64, 50, 0]);
    assert_eq!(arrival_color(1.0, highlight, background), [0, 0, 0]);
    assert_eq!(arrival_color(2.0, highlight, background), [0, 0, 0]);
}

#[test]
fn the_terminal_title_has_the_pending_count_and_marks_a_failed_poll() {
    use review_queue::app::terminal_title;

    assert_eq!(terminal_title(&App::default()), "review-queue");
    let mut app = app_with(&[1, 2, 3]);
    assert_eq!(terminal_title(&app), "review-queue (3)");
    assert_eq!(terminal_title(&app_with(&[])), "review-queue (0)");

    app.handle(
        Event::PollFailed {
            error: "offline".into(),
        },
        &now(),
    );
    assert_eq!(terminal_title(&app), "review-queue (3, error)");

    // Snoozed requests are not pending.
    press(&mut app, Action::Dismiss);
    assert_eq!(terminal_title(&app), "review-queue (2, error)");
}
