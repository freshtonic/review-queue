//! The state of the TUI, and how it changes for each user action or poll result.
//! It has no terminal dependencies.

use jiff::civil::Weekday;
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::queue::{LatenessThresholds, QueueItem};
use crate::snooze::{
    Snooze, SnoozeChoice, SnoozedItem, WorkingTime, apply_snoozes, parse_duration, snooze_until,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Down,
    Up,
    /// Open the selected pull request: for a re-review, the changes since the last review.
    Open,
    /// Open the whole selected pull request.
    OpenFull,
    /// Copy the link that `Open` opens.
    Copy,
    Refresh,
    ToggleHelp,
    Quit,
    /// Snooze the selected request for 1 working day.
    Dismiss,
    /// Open the snooze picker.
    Snooze,
    /// Choose the snooze picker choice with this number (from 1).
    Choose(usize),
    /// Type a character of a custom duration.
    Type(char),
    Backspace,
    /// Accept the custom duration.
    Confirm,
    /// Close the snooze picker.
    Cancel,
    /// Show or close the snoozed view.
    ToggleSnoozedView,
    /// In the snoozed view, end the Snooze of the selected request.
    Unsnooze,
    /// Move the focus between the list and the detail panel.
    ToggleFocus,
    /// Show or close the empty-queue image, to preview it.
    ToggleAttempt,
    /// Scroll the description 10 lines down, in the detail panel.
    PageDown,
    PageUp,
    /// Make the detail panel 10% of the height taller, or shorter.
    GrowPanel,
    ShrinkPanel,
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
    /// Copy this text to the clipboard.
    Copy(String),
    Refresh,
    Quit,
    /// Write these Snoozes to the Snooze file.
    SaveSnoozes(Vec<Snooze>),
}

/// A choice in the snooze picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerChoice {
    pub label: String,
    /// The end time of the Snooze. `None` for the custom duration.
    pub until: Option<Timestamp>,
    choice: Option<SnoozeChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    pub choices: Vec<PickerChoice>,
    /// The custom duration, while the user types it.
    pub input: Option<String>,
    /// Why the custom duration is not valid.
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct App {
    /// The Queue without the snoozed requests.
    pub queue: Vec<QueueItem>,
    /// The snoozed requests.
    pub snoozed: Vec<SnoozedItem>,
    /// The index of the selected item in `queue`, or in `snoozed` in the snoozed view.
    pub selected: usize,
    /// The time of the last good poll.
    pub updated_at: Option<Timestamp>,
    /// The error from the last poll, if it failed.
    pub error: Option<String>,
    pub show_help: bool,
    /// The snooze picker, while it is open.
    pub picker: Option<Picker>,
    /// True while the snoozed view shows.
    pub show_snoozed: bool,
    /// True while the empty-queue image shows as a preview.
    pub show_attempt: bool,
    /// True while the detail panel has the focus: up and down scroll the description.
    pub panel_focused: bool,
    /// The change to the panel height from the default 50%, in steps of 10%.
    panel_steps: i16,
    /// The first description line that shows, in the detail panel.
    pub description_scroll: usize,
    /// From the last draw: whether the panel shows, and the largest description scroll.
    panel_shown: bool,
    max_scroll: usize,
    /// The pull request whose description `description_scroll` is for.
    scrolled: Option<(String, u64)>,
    /// The rows that arrived at a poll, and when.
    arrivals: Vec<((String, u64), Timestamp)>,
    pub working: WorkingTime,
    /// When a request becomes Stale and Overdue.
    pub thresholds: LatenessThresholds,
    /// When the user last copied a link, for the status bar notice.
    copied_at: Option<Timestamp>,
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
        let effect = self.handle_event(event, now);
        // A new selection starts its description at the top.
        let selected = self
            .selected_item()
            .map(|i| (i.pull_request.repo.clone(), i.pull_request.number));
        if selected != self.scrolled {
            self.scrolled = selected;
            self.description_scroll = 0;
        }
        effect
    }

    fn handle_event(&mut self, event: Event, now: &Zoned) -> Effect {
        if let Event::Action(action) = event {
            if self.panel_focused && self.picker.is_none() && !self.show_help {
                let scroll = self.description_scroll;
                let lines = match action {
                    Action::Down => Some(scroll + 1),
                    Action::Up => Some(scroll.saturating_sub(1)),
                    Action::PageDown => Some(scroll + 10),
                    Action::PageUp => Some(scroll.saturating_sub(10)),
                    _ => None,
                };
                if let Some(lines) = lines {
                    self.description_scroll = lines.min(self.max_scroll);
                    return Effect::None;
                }
            }
            if self.picker.is_some() {
                return self.handle_picker(action, now);
            }
            if (self.show_help || self.show_attempt)
                && matches!(
                    action,
                    Action::Down
                        | Action::Up
                        | Action::Open
                        | Action::OpenFull
                        | Action::Dismiss
                        | Action::Snooze
                )
            {
                return Effect::None;
            }
        }
        match event {
            Event::Action(Action::ToggleHelp) => self.show_help = !self.show_help,
            Event::Action(Action::Down) => {
                self.selected = (self.selected + 1).min(self.view_len().saturating_sub(1));
            }
            Event::Action(Action::Up) => self.selected = self.selected.saturating_sub(1),
            Event::Action(Action::Open) => {
                if let Some(item) = self.selected_item() {
                    return Effect::OpenUrl(review_url(item));
                }
            }
            Event::Action(Action::Copy) => {
                if let Some(item) = self.selected_item() {
                    let url = review_url(item);
                    self.copied_at = Some(now.timestamp());
                    return Effect::Copy(url);
                }
            }
            Event::Action(Action::OpenFull) => {
                if let Some(item) = self.selected_item() {
                    return Effect::OpenUrl(pull_request_url(item));
                }
            }
            Event::Action(Action::Refresh) => return Effect::Refresh,
            Event::Action(Action::Quit) => return Effect::Quit,
            Event::Action(Action::Snooze) => {
                if self.selected_item().is_some() {
                    self.picker = Some(self.new_picker(now));
                }
            }
            Event::Action(
                Action::Choose(_)
                | Action::Type(_)
                | Action::Backspace
                | Action::Confirm
                | Action::Cancel,
            ) => {}
            Event::Action(Action::ToggleFocus) => {
                self.panel_focused = !self.panel_focused && self.panel_shown;
            }
            Event::Action(Action::ToggleAttempt) => self.show_attempt = !self.show_attempt,
            Event::Action(Action::PageDown | Action::PageUp) => {}
            Event::Action(Action::GrowPanel) => self.panel_steps = (self.panel_steps + 1).min(3),
            Event::Action(Action::ShrinkPanel) => self.panel_steps = (self.panel_steps - 1).max(-3),
            Event::Action(Action::ToggleSnoozedView) => {
                self.show_snoozed = !self.show_snoozed;
                self.selected = 0;
            }
            Event::Action(Action::Unsnooze) => {
                if self.show_snoozed
                    && let Some(item) = self.selected_item()
                {
                    let (repo, number) = (item.pull_request.repo.clone(), item.pull_request.number);
                    self.snoozes
                        .retain(|s| !(s.repo == repo && s.number == number));
                    self.refresh_view(now.timestamp());
                    return Effect::SaveSnoozes(self.snoozes.clone());
                }
            }
            Event::Action(Action::Dismiss) => {
                return self.snooze_selected(SnoozeChoice::Dismiss, now);
            }
            Event::PollSucceeded { queue, at } => {
                // At the first poll, every row is new, so none is an arrival.
                let before: Option<Vec<(String, u64)>> = self
                    .updated_at
                    .map(|_| self.queue.iter().map(key).collect());
                self.updated_at = Some(at);
                self.error = None;
                self.all = queue;
                let snoozes_before = self.snoozes.clone();
                self.refresh_view(at);
                self.arrivals
                    .retain(|(_, started)| at.duration_since(*started) < ARRIVAL);
                if let Some(before) = before {
                    for item in &self.queue {
                        if !before.contains(&key(item)) {
                            self.arrivals.push((key(item), at));
                        }
                    }
                }
                if self.snoozes != snoozes_before {
                    return Effect::SaveSnoozes(self.snoozes.clone());
                }
            }
            Event::PollFailed { error } => self.error = Some(error),
        }
        Effect::None
    }

    /// The height of the detail panel, in percent of the body: 20 to 80, 50 at the start.
    pub fn panel_percent(&self) -> u16 {
        (50 + 10 * self.panel_steps) as u16
    }

    /// The selected item in the view that shows: the Queue, or the snoozed requests.
    pub fn selected_item(&self) -> Option<&QueueItem> {
        if self.show_snoozed {
            self.snoozed.get(self.selected).map(|s| &s.item)
        } else {
            self.queue.get(self.selected)
        }
    }

    /// The rows that arrived at a recent poll, and how far their effect is, from 0 (just
    /// arrived) to 1. A finished effect is not in the list.
    pub fn arrival_progress(&self, now: Timestamp) -> Vec<((String, u64), f32)> {
        self.arrivals
            .iter()
            .map(|(key, started)| {
                let elapsed = now.duration_since(*started).as_secs_f32();
                (key.clone(), elapsed / ARRIVAL.as_secs_f32())
            })
            .filter(|(_, progress)| (0.0..1.0).contains(progress))
            .collect()
    }

    /// After each draw: whether the detail panel shows, and the largest description scroll.
    pub fn after_draw(&mut self, panel_shown: bool, max_scroll: usize) {
        self.panel_shown = panel_shown;
        self.max_scroll = max_scroll;
        self.panel_focused &= panel_shown;
        self.description_scroll = self.description_scroll.min(max_scroll);
    }

    /// The end of the Snooze of the selected request, in the snoozed view.
    pub fn selected_snoozed_until(&self) -> Option<Timestamp> {
        self.show_snoozed
            .then(|| self.snoozed.get(self.selected))
            .flatten()
            .map(|s| s.until)
    }

    fn view_len(&self) -> usize {
        if self.show_snoozed {
            self.snoozed.len()
        } else {
            self.queue.len()
        }
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
        if self
            .copied_at
            .is_some_and(|at| now.duration_since(at) < COPIED_NOTICE)
        {
            status.push_str(" · copied the link");
        }
        status
    }

    /// The actions while the snooze picker is open. Other actions do nothing.
    fn handle_picker(&mut self, action: Action, now: &Zoned) -> Effect {
        let Some(picker) = self.picker.as_mut() else {
            return Effect::None;
        };
        match (&mut picker.input, action) {
            (_, Action::Quit) => return Effect::Quit,
            (_, Action::Cancel) => self.picker = None,
            (None, Action::Choose(number)) => match picker.choices.get(number.wrapping_sub(1)) {
                Some(PickerChoice {
                    choice: Some(choice),
                    ..
                }) => {
                    let choice = *choice;
                    self.picker = None;
                    return self.snooze_selected(choice, now);
                }
                Some(_) => picker.input = Some(String::new()),
                None => {}
            },
            (Some(input), Action::Type(c)) => input.push(c),
            (Some(input), Action::Backspace) => {
                input.pop();
            }
            (Some(input), Action::Confirm) => match parse_duration(input) {
                Some(duration) => {
                    self.picker = None;
                    return self.snooze_selected(SnoozeChoice::For(duration), now);
                }
                None => {
                    picker.error = Some("Type a duration such as 45m, 3h, 2d, or 1h30m.".into())
                }
            },
            _ => {}
        }
        Effect::None
    }

    fn new_picker(&self, now: &Zoned) -> Picker {
        let choice = |label: String, choice: SnoozeChoice| PickerChoice {
            label,
            until: Some(snooze_until(choice, now, &self.working)),
            choice: Some(choice),
        };
        let mut choices = vec![
            choice(
                "1 hour".into(),
                SnoozeChoice::For(SignedDuration::from_hours(1)),
            ),
            choice(
                "3 hours".into(),
                SnoozeChoice::For(SignedDuration::from_hours(3)),
            ),
            choice("Next morning".into(), SnoozeChoice::NextMorning),
        ];
        // One choice for each of the next 5 working days, the soonest first.
        let mut before_cob: Vec<PickerChoice> = self
            .working
            .days
            .iter()
            .map(|&day| {
                choice(
                    format!("Before COB {}", short_weekday(day)),
                    SnoozeChoice::BeforeCob(day),
                )
            })
            .collect();
        before_cob.sort_by_key(|c| c.until);
        choices.extend(before_cob.into_iter().take(5));
        choices.push(PickerChoice {
            label: "Custom…".into(),
            until: None,
            choice: None,
        });
        Picker {
            choices,
            input: None,
            error: None,
        }
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
    /// selection on the same pull request if it is still in the view that shows.
    fn refresh_view(&mut self, now: Timestamp) {
        let key = |i: &QueueItem| (i.pull_request.repo.clone(), i.pull_request.number);
        let selected_key = self.selected_item().map(key);
        let outcome = apply_snoozes(self.all.clone(), &self.snoozes, now);
        self.queue = outcome.visible;
        self.snoozed = outcome.snoozed;
        self.snoozes = outcome.kept;
        let view: Vec<&QueueItem> = if self.show_snoozed {
            self.snoozed.iter().map(|s| &s.item).collect()
        } else {
            self.queue.iter().collect()
        };
        self.selected = selected_key
            .and_then(|k| view.iter().position(|&i| key(i) == k))
            .unwrap_or(self.selected)
            .min(view.len().saturating_sub(1));
    }
}

fn short_weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

/// How long the status bar says that a link was copied.
const COPIED_NOTICE: SignedDuration = SignedDuration::from_secs(3);

/// The length of the arrival effect.
const ARRIVAL: jiff::SignedDuration = jiff::SignedDuration::from_secs(1);

/// The identity of a pull request in the Queue.
fn key(item: &QueueItem) -> (String, u64) {
    (item.pull_request.repo.clone(), item.pull_request.number)
}

fn pull_request_url(item: &QueueItem) -> String {
    let pr = &item.pull_request;
    format!("https://github.com/{}/pull/{}", pr.repo, pr.number)
}

/// The page to review the item on: for a re-review with new commits, the changes since
/// the user's last review; otherwise, the whole pull request.
pub fn review_url(item: &QueueItem) -> String {
    let details = &item.pull_request.details;
    match &details.reviewed_commit {
        Some(reviewed) if *reviewed != details.head_commit && !details.head_commit.is_empty() => {
            format!(
                "{}/files/{reviewed}..{}",
                pull_request_url(item),
                details.head_commit
            )
        }
        _ => pull_request_url(item),
    }
}

/// The terminal title: the number of pending requests, so that it shows in a tab or
/// pane title while the Queue is out of view. Before the first good poll, no count.
pub fn terminal_title(app: &App) -> String {
    match (app.updated_at, &app.error) {
        (None, _) => "review-queue".into(),
        (Some(_), None) => format!("review-queue ({})", app.queue.len()),
        (Some(_), Some(_)) => format!("review-queue ({}, error)", app.queue.len()),
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

/// The background color of an arriving row at `progress` (0 to 1) of its effect.
/// The fade is ease-out: fast at the start, slow at the end.
pub fn arrival_color(progress: f32, highlight: [u8; 3], background: [u8; 3]) -> [u8; 3] {
    let p = progress.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - p) * (1.0 - p);
    let mut color = [0; 3];
    for i in 0..3 {
        let (from, to) = (highlight[i] as f32, background[i] as f32);
        color[i] = (from + (to - from) * eased).round() as u8;
    }
    color
}
