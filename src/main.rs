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

use ratatui_image::picker::{Picker, ProtocolType};
use review_queue::app::{Action, App, Effect, Event};
use review_queue::config::{self, Config};
use review_queue::snooze::WorkingTime;
use review_queue::ui::{self, ConfigView, Graphics};
use review_queue::{auth, avatar, github, queue, snooze};

/// A terminal UI that shows the GitHub pull requests that are waiting for your review.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The time between two polls, in seconds. This overrides the config file.
    #[arg(long, value_name = "SECONDS")]
    interval: Option<u64>,

    /// Do not use raster graphics.
    #[arg(long)]
    no_images: bool,

    /// Print the Queue as tab-separated lines, then exit.
    #[arg(long, conflicts_with = "count")]
    once: bool,

    /// Print the number of pending requests, then exit.
    #[arg(long)]
    count: bool,
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
    let working = match config.working_time() {
        Ok(working) => working,
        Err(e) => {
            eprintln!("review-queue: {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

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

    if cli.once || cli.count {
        let snapshot = match github::fetch(&reqwest::Client::new(), &token).await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                eprintln!("review-queue: {e}");
                return ExitCode::FAILURE;
            }
        };
        if snapshot.next_page.is_some() {
            eprintln!(
                "review-queue: not all requests are shown (over {})",
                github::MAX_PAGES * 100
            );
        }
        let now = Timestamp::now();
        let all = queue::build_queue(
            &snapshot.pull_requests,
            &snapshot.viewer,
            &config.watched_teams,
            &config.exclusions(),
        );
        let visible = snooze::apply_snoozes(all, &snooze::load(&snooze::store_path()), now).visible;
        if cli.count {
            println!("{}", visible.len());
        } else {
            print!("{}", queue::plain_lines(&visible, now));
        }
        return ExitCode::SUCCESS;
    }

    let mut view = ConfigView {
        path: tilde(&path),
        interval_secs: config.interval_secs,
        watched_teams: config.watched_teams.clone(),
        working_time: format!(
            "{}–{}, {}",
            config.work_start,
            config.work_end,
            config.work_days.join(" ")
        ),
        aging: format!(
            "stale after {}h, overdue after {}h (working time)",
            config.stale_after_hours, config.overdue_after_hours
        ),
        excluded: {
            let all: Vec<String> = config
                .exclude_repos
                .iter()
                .chain(&config.exclude_authors)
                .cloned()
                .collect();
            if all.is_empty() {
                "(none)".into()
            } else {
                all.join(", ")
            }
        },
        snooze_file: tilde(&snooze::store_path()),
        token_source,
        graphics: "",
    };

    let terminal = ratatui::init();
    // Capture the mouse, so that a wheel or trackpad over the detail panel scrolls the
    // description. Without it, the terminal sends the wheel as arrow keys.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    // Keep the terminal's title on its title stack (XTWINOPS), to put it back at exit.
    print!("\x1b[22;0t");
    // Ask the terminal for its graphics protocol and font size. This needs the
    // terminal in raw mode, so it comes after `ratatui::init`.
    let picker = if cli.no_images {
        None
    } else {
        Picker::from_query_stdio()
            .ok()
            .filter(|p| p.protocol_type() != ProtocolType::Halfblocks)
    };
    let graphics = Graphics::new(picker, avatar::cache_dir());
    view.graphics = graphics.description();
    let result = run(terminal, config, working, token, view, graphics).await;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
    print!("\x1b[23;0t");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("review-queue: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The path, with `~` for the home folder.
fn tilde(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    match home.and_then(|home| path.strip_prefix(home).ok().map(|p| p.to_path_buf())) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
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
    working: WorkingTime,
    token: String,
    view: ConfigView,
    mut graphics: Graphics,
) -> std::io::Result<()> {
    let client = reqwest::Client::new();
    let (results, mut incoming) = mpsc::channel::<Event>(4);
    let mut keys = EventStream::new();
    let mut poll_timer = tokio::time::interval(Duration::from_secs(config.interval_secs));
    // Redraw each second, so that the ages in the status bar and the rows stay current.
    let mut redraw_timer = tokio::time::interval(Duration::from_secs(1));
    // Redraw 20 times a second while a row's arrival effect runs.
    let mut fade_timer = tokio::time::interval(Duration::from_millis(50));
    // Avatar downloads: the login, and whether the file was written.
    let (avatars_written, mut avatar_written) = mpsc::channel::<(String, bool)>(16);
    let mut avatars_in_flight: std::collections::HashSet<String> = Default::default();
    let mut app = App::new(working, snooze::load(&snooze::store_path()));
    app.thresholds = config.lateness_thresholds();
    let mut poll_in_flight = false;

    let start_poll = |in_flight: &mut bool| {
        if *in_flight {
            return;
        }
        *in_flight = true;
        let client = client.clone();
        let token = token.clone();
        let watched_teams = config.watched_teams.clone();
        let exclusions = config.exclusions();
        let results = results.clone();
        tokio::spawn(async move {
            let event = match github::fetch(&client, &token).await {
                Ok(snapshot) => Event::PollSucceeded {
                    queue: queue::build_queue(
                        &snapshot.pull_requests,
                        &snapshot.viewer,
                        &watched_teams,
                        &exclusions,
                    ),
                    at: Timestamp::now(),
                    incomplete: snapshot.next_page.is_some(),
                },
                Err(error) => Event::PollFailed { error },
            };
            let _ = results.send(event).await;
        });
    };

    let mut title = String::new();
    loop {
        let new_title = review_queue::app::terminal_title(&app);
        if new_title != title {
            crossterm::execute!(std::io::stdout(), crossterm::terminal::SetTitle(&new_title))?;
            title = new_title;
        }
        let mut drawn = ui::Drawn::default();
        terminal
            .draw(|frame| drawn = ui::draw(frame, &app, Timestamp::now(), &view, &mut graphics))?;
        app.after_draw(drawn.panel_shown, drawn.max_scroll);
        let panel_area = drawn.panel_area;

        let effect = tokio::select! {
            _ = poll_timer.tick() => {
                start_poll(&mut poll_in_flight);
                Effect::None
            }
            Some(event) = incoming.recv() => {
                poll_in_flight = false;
                if let Event::PollSucceeded { queue, .. } = &event {
                    for item in queue {
                        let author = &item.pull_request.author;
                        let Some(url) = &author.avatar_url else { continue };
                        let path = avatar::path_for(&avatar::cache_dir(), &author.login);
                        if avatar::freshness(&path, std::time::SystemTime::now()) == avatar::Freshness::Fresh
                            || !avatars_in_flight.insert(author.login.clone())
                        {
                            continue;
                        }
                        let (client, url, login, done) =
                            (client.clone(), url.clone(), author.login.clone(), avatars_written.clone());
                        tokio::spawn(async move {
                            let written = download(&client, &url, &path).await.is_ok();
                            let _ = done.send((login, written)).await;
                        });
                    }
                }
                app.handle(event, &Zoned::now())
            }
            Some((login, written)) = avatar_written.recv() => {
                // A failed download is tried again at the next poll.
                avatars_in_flight.remove(&login);
                if written {
                    graphics.avatar_changed(&login);
                }
                Effect::None
            }
            Some(Ok(event)) = keys.next() => {
                let action = match event {
                    TermEvent::Key(key) => action_for(key, &app),
                    TermEvent::Mouse(mouse) => scroll_action(mouse, panel_area, &app),
                    _ => None,
                };
                match action {
                    Some(action) => app.handle(Event::Action(action), &Zoned::now()),
                    None => Effect::None,
                }
            }
            _ = redraw_timer.tick() => Effect::None,
            _ = fade_timer.tick(), if !app.arrival_progress(Timestamp::now()).is_empty() => Effect::None,
        };

        match effect {
            Effect::None => {}
            Effect::OpenUrl(url) => open_in_browser(&url),
            Effect::Copy(text) => copy_to_clipboard(&text),
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
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::Quit);
    }
    // While the user types a custom duration, the keys are text.
    if let Some(picker) = &app.picker {
        return match (key.code, picker.input.is_some()) {
            (KeyCode::Esc, _) => Some(Action::Cancel),
            (KeyCode::Enter, true) => Some(Action::Confirm),
            (KeyCode::Backspace, true) => Some(Action::Backspace),
            (KeyCode::Char(c), true) => Some(Action::Type(c)),
            (KeyCode::Char(c), false) => c.to_digit(10).map(|n| Action::Choose(n as usize)),
            _ => None,
        };
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter | KeyCode::Char('o') => Some(Action::Open),
        KeyCode::Char('O') => Some(Action::OpenFull),
        KeyCode::Char('y') => Some(Action::Copy),
        KeyCode::Char('r') => Some(Action::Refresh),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Esc if app.show_help => Some(Action::ToggleHelp),
        KeyCode::Esc if app.show_attempt => Some(Action::ToggleAttempt),
        KeyCode::Esc if app.panel_focused => Some(Action::ToggleFocus),
        KeyCode::Char('a') => Some(Action::ToggleAttempt),
        KeyCode::Esc if app.show_snoozed => Some(Action::ToggleSnoozedView),
        KeyCode::Tab => Some(Action::ToggleFocus),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::Char('+' | '=') => Some(Action::GrowPanel),
        KeyCode::Char('-' | '_') => Some(Action::ShrinkPanel),
        KeyCode::Char(' ') if app.panel_focused => Some(Action::PageDown),
        KeyCode::Char('s') => Some(Action::Snooze),
        KeyCode::Char('d') => Some(Action::Dismiss),
        KeyCode::Char('z') => Some(Action::ToggleSnoozedView),
        KeyCode::Char('u') => Some(Action::Unsnooze),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

/// A mouse wheel or trackpad: over the detail panel, it scrolls the description;
/// elsewhere, it moves the selection.
fn scroll_action(
    mouse: crossterm::event::MouseEvent,
    panel_area: Option<ratatui::layout::Rect>,
    app: &App,
) -> Option<Action> {
    use crossterm::event::MouseEventKind;
    if app.picker.is_some() || app.show_help {
        return None;
    }
    let over_panel = panel_area
        .is_some_and(|area| area.contains(ratatui::layout::Position::new(mouse.column, mouse.row)));
    match (mouse.kind, over_panel) {
        (MouseEventKind::ScrollDown, true) => Some(Action::ScrollDescriptionDown),
        (MouseEventKind::ScrollUp, true) => Some(Action::ScrollDescriptionUp),
        (MouseEventKind::ScrollDown, false) if !app.panel_focused => Some(Action::Down),
        (MouseEventKind::ScrollUp, false) if !app.panel_focused => Some(Action::Up),
        _ => None,
    }
}

/// Downloads an avatar to `path`. It writes a temporary file and renames it, so that a
/// half-written file never shows.
async fn download(
    client: &reqwest::Client,
    url: &str,
    path: &std::path::Path,
) -> Result<(), String> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "review-queue")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("png.tmp");
    std::fs::write(&temporary, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}

/// Copies the text with OSC 52, so that it works over SSH and in multiplexers such as
/// herdr and tmux, when they pass the sequence through.
fn copy_to_clipboard(text: &str) {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    print!("\x1b]52;c;{encoded}\x07");
    let _ = std::io::Write::flush(&mut std::io::stdout());
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
