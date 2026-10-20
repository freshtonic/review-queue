use std::process::{Command, ExitCode};
use std::time::Duration;

use clap::Parser;
use crossterm::event::{
    Event as TermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures::StreamExt;
use jiff::{Timestamp, Zoned};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use review_queue::app::{Action, App, Effect, Event};
use review_queue::config::{self, Config};
use review_queue::snooze::WorkingTime;
use review_queue::ui::{self, ConfigView};
use review_queue::{auth, github, queue, snooze};

/// A terminal UI that shows the GitHub pull requests that are waiting for your review.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The time between two polls, in seconds. This overrides the config file.
    #[arg(long, value_name = "SECONDS")]
    interval: Option<u64>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let path = config::path();
    let mut config = match config::load(&path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("review-queue: cannot read the config file: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(interval) = cli.interval {
        config.interval_secs = interval;
    }
    config.interval_secs = config.interval_secs.max(1);

    let env_token = std::env::var("GITHUB_TOKEN").ok();
    let token_source = if env_token.as_deref().is_some_and(|t| !t.trim().is_empty()) {
        "GITHUB_TOKEN"
    } else {
        "gh auth token"
    };
    let token = match auth::resolve_token(env_token, gh_auth_token) {
        Ok(token) => token,
        Err(e) => {
            eprintln!("review-queue: {}", e.0);
            return ExitCode::FAILURE;
        }
    };

    let view = ConfigView {
        path: path.display().to_string(),
        interval_secs: config.interval_secs,
        watched_teams: config.watched_teams.clone(),
        token_source,
    };

    let terminal = ratatui::init();
    let result = run(terminal, config, token, view).await;
    ratatui::restore();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("review-queue: {e}");
            ExitCode::FAILURE
        }
    }
}

fn gh_auth_token() -> Result<String, String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|e| format!("cannot run gh: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

async fn run(
    mut terminal: DefaultTerminal,
    config: Config,
    token: String,
    view: ConfigView,
) -> std::io::Result<()> {
    let client = reqwest::Client::new();
    let (results, mut incoming) = mpsc::channel::<Event>(4);
    let mut keys = EventStream::new();
    let mut poll_timer = tokio::time::interval(Duration::from_secs(config.interval_secs));
    // Redraw each second, so that the ages in the status bar and the rows stay current.
    let mut redraw_timer = tokio::time::interval(Duration::from_secs(1));
    let mut app = App::new(WorkingTime::default(), snooze::load(&snooze::store_path()));
    let mut poll_in_flight = false;

    let start_poll = |in_flight: &mut bool| {
        if *in_flight {
            return;
        }
        *in_flight = true;
        let client = client.clone();
        let token = token.clone();
        let watched_teams = config.watched_teams.clone();
        let results = results.clone();
        tokio::spawn(async move {
            let event = match github::fetch(&client, &token).await {
                Ok(snapshot) => Event::PollSucceeded {
                    queue: queue::build_queue(
                        &snapshot.pull_requests,
                        &snapshot.viewer,
                        &watched_teams,
                    ),
                    at: Timestamp::now(),
                },
                Err(error) => Event::PollFailed { error },
            };
            let _ = results.send(event).await;
        });
    };

    loop {
        terminal.draw(|frame| ui::draw(frame, &app, Timestamp::now(), &view))?;

        let effect = tokio::select! {
            _ = poll_timer.tick() => {
                start_poll(&mut poll_in_flight);
                Effect::None
            }
            Some(event) = incoming.recv() => {
                poll_in_flight = false;
                app.handle(event, &Zoned::now())
            }
            Some(Ok(TermEvent::Key(key))) = keys.next() => match action_for(key, &app) {
                Some(action) => app.handle(Event::Action(action), &Zoned::now()),
                None => Effect::None,
            },
            _ = redraw_timer.tick() => Effect::None,
        };

        match effect {
            Effect::None => {}
            Effect::OpenUrl(url) => open_in_browser(&url),
            Effect::Refresh => start_poll(&mut poll_in_flight),
            Effect::Quit => return Ok(()),
            Effect::SaveSnoozes(snoozes) => {
                if let Err(e) = snooze::save(&snooze::store_path(), &snoozes) {
                    app.error = Some(format!("cannot save snoozes: {e}"));
                }
            }
        }
    }
}

fn action_for(key: KeyEvent, app: &App) -> Option<Action> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter => Some(Action::Open),
        KeyCode::Char('r') => Some(Action::Refresh),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Char('d') => Some(Action::Dismiss),
        KeyCode::Esc if app.show_help => Some(Action::ToggleHelp),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

fn open_in_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = Command::new(opener)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
