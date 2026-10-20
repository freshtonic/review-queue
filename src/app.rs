//! The state of the TUI, and how it changes for each user action or poll result.
//! It has no terminal dependencies.

use jiff::Timestamp;

use crate::queue::QueueItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Down,
    Up,
    Open,
    Refresh,
    ToggleHelp,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Action(Action),
    PollSucceeded { queue: Vec<QueueItem>, at: Timestamp },
    PollFailed { error: String },
}

/// What the shell must do after an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    OpenUrl(String),
    Refresh,
    Quit,
}

#[derive(Debug, Default)]
pub struct App {
    pub queue: Vec<QueueItem>,
    /// The index of the selected item in `queue`.
    pub selected: usize,
    /// The time of the last good poll.
    pub updated_at: Option<Timestamp>,
    /// The error from the last poll, if it failed.
    pub error: Option<String>,
}

impl App {
    pub fn handle(&mut self, event: Event) -> Effect {
        match event {
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
            Event::PollSucceeded { queue, at } => {
                self.updated_at = Some(at);
                self.error = None;
                let key = |i: &QueueItem| (i.pull_request.repo.clone(), i.pull_request.number);
                let selected_key = self.selected_item().map(key);
                self.selected = selected_key
                    .and_then(|k| queue.iter().position(|i| key(i) == k))
                    .unwrap_or(self.selected)
                    .min(queue.len().saturating_sub(1));
                self.queue = queue;
            }
            Event::PollFailed { error } => self.error = Some(error),
            _ => {}
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
        if let Some(error) = &self.error {
            status.push_str(&format!(" · error: {error}"));
        }
        status
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
