use jiff::Timestamp;
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
        },
        tier: PriorityTier::PersonalFirstReview,
        requested_at: at("2026-10-01T09:00:00Z"),
        team: None,
    }
}

fn app_with(numbers: &[u64]) -> App {
    let mut app = App::default();
    app.handle(Event::PollSucceeded {
        queue: numbers.iter().map(|&n| item(n)).collect(),
        at: at("2026-10-20T09:00:00Z"),
    });
    app
}

fn press(app: &mut App, action: Action) -> Effect {
    app.handle(Event::Action(action))
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
