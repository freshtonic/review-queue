//! The state of the TUI, and how it changes for each user action or poll result.
//! It has no terminal dependencies.

use jiff::{Timestamp, Zoned};

use crate::queue::QueueItem;
use crate::snooze::{Snooze, SnoozeChoice, SnoozedItem, WorkingTime, apply_snoozes, snooze_until};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Down,
    Up,
    Open,
    Refresh,
    ToggleHelp,
    Quit,
    /// Snooze the selected request for 1 working day.
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Action(Action),
    PollSucceeded {
        queue: Vec<QueueItem>,
        at: Timestamp,
    },
    PollFailed {
        error: String,
    },
}

/// What the shell must do after an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    OpenUrl(String),
    Refresh,
    Quit,
    /// Write these Snoozes to the Snooze file.
    SaveSnoozes(Vec<Snooze>),
}

#[derive(Debug, Default)]
pub struct App {
    /// The Queue without the snoozed requests.
    pub queue: Vec<QueueItem>,
    /// The snoozed requests.
    pub snoozed: Vec<SnoozedItem>,
    /// The index of the selected item in `queue`.
    pub selected: usize,
    /// The time of the last good poll.
    pub updated_at: Option<Timestamp>,
    /// The error from the last poll, if it failed.
    pub error: Option<String>,
    pub show_help: bool,
    pub working: WorkingTime,
    /// The whole Queue from the last good poll, with the snoozed requests.
    all: Vec<QueueItem>,
    snoozes: Vec<Snooze>,
}

impl App {
    pub fn new(working: WorkingTime, snoozes: Vec<Snooze>) -> Self {
        Self {
            working,
            snoozes,
            ..Self::default()
        }
    }

    pub fn handle(&mut self, event: Event, now: &Zoned) -> Effect {
        if self.show_help
            && let Event::Action(Action::Down | Action::Up | Action::Open | Action::Dismiss) = event
        {
            return Effect::None;
        }
        match event {
            Event::Action(Action::ToggleHelp) => self.show_help = !self.show_help,
            Event::Action(Action::Down) => {
                self.selected = (self.selected + 1).min(self.queue.len().saturating_sub(1));
            }
            Event::Action(Action::Up) => self.selected = self.selected.saturating_sub(1),
            Event::Action(Action::Open) => {
                if let Some(item) = self.selected_item() {
                    let pr = &item.pull_request;
                    return Effect::OpenUrl(format!(
                        "https://github.com/{}/pull/{}",
                        pr.repo, pr.number
                    ));
                }
            }
            Event::Action(Action::Refresh) => return Effect::Refresh,
            Event::Action(Action::Quit) => return Effect::Quit,
            Event::Action(Action::Dismiss) => {
                return self.snooze_selected(SnoozeChoice::Dismiss, now);
            }
            Event::PollSucceeded { queue, at } => {
                self.updated_at = Some(at);
                self.error = None;
                self.all = queue;
                let before = self.snoozes.clone();
                self.refresh_view(at);
                if self.snoozes != before {
                    return Effect::SaveSnoozes(self.snoozes.clone());
                }
            }
            Event::PollFailed { error } => self.error = Some(error),
        }
        Effect::None
    }

    pub fn selected_item(&self) -> Option<&QueueItem> {
        self.queue.get(self.selected)
    }

    pub fn status(&self, now: Timestamp) -> String {
        let Some(updated_at) = self.updated_at else {
            return match &self.error {
                Some(error) => format!("error: {error}"),
                None => "loading…".into(),
            };
        };
        let mut status = format!(
            "updated {} ago · {} pending",
            short_duration(updated_at, now),
            self.queue.len()
        );
        if !self.snoozed.is_empty() {
            status.push_str(&format!(" · {} snoozed", self.snoozed.len()));
        }
        if let Some(error) = &self.error {
            status.push_str(&format!(" · error: {error}"));
        }
        status
    }

    fn snooze_selected(&mut self, choice: SnoozeChoice, now: &Zoned) -> Effect {
        let Some(item) = self.selected_item() else {
            return Effect::None;
        };
        let pr = &item.pull_request;
        let snooze = Snooze {
            repo: pr.repo.clone(),
            number: pr.number,
            started_at: now.timestamp(),
            until: snooze_until(choice, now, &self.working),
        };
        self.snoozes
            .retain(|s| !(s.repo == snooze.repo && s.number == snooze.number));
        self.snoozes.push(snooze);
        self.refresh_view(now.timestamp());
        Effect::SaveSnoozes(self.snoozes.clone())
    }

    /// Splits the whole Queue into the visible and the snoozed requests, and keeps the
    /// selection on the same pull request if it is still visible.
    fn refresh_view(&mut self, now: Timestamp) {
        let key = |i: &QueueItem| (i.pull_request.repo.clone(), i.pull_request.number);
        let selected_key = self.selected_item().map(key);
        let outcome = apply_snoozes(self.all.clone(), &self.snoozes, now);
        self.selected = selected_key
            .and_then(|k| outcome.visible.iter().position(|i| key(i) == k))
            .unwrap_or(self.selected)
            .min(outcome.visible.len().saturating_sub(1));
        self.queue = outcome.visible;
        self.snoozed = outcome.snoozed;
        self.snoozes = outcome.kept;
    }
}

/// The time from `since` to `now`, in its largest whole unit: "12s", "5m", "3h", "2d".
pub fn short_duration(since: Timestamp, now: Timestamp) -> String {
    let seconds = now.duration_since(since).as_secs().max(0);
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3_600 => format!("{}m", seconds / 60),
        3_600..86_400 => format!("{}h", seconds / 3_600),
        _ => format!("{}d", seconds / 86_400),
    }
}
