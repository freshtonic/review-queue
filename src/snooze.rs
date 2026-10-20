//! Snooze: when a Snooze ends.

use jiff::civil::{Time, Weekday};
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::queue::QueueItem;

/// The user's working hours and working days, in the local time zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingTime {
    pub start: Time,
    pub end: Time,
    pub days: Vec<Weekday>,
}

impl Default for WorkingTime {
    fn default() -> Self {
        Self {
            start: Time::constant(9, 0, 0, 0),
            end: Time::constant(17, 30, 0, 0),
            days: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
        }
    }
}

/// The choices in the snooze picker, and dismiss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnoozeChoice {
    /// A wall-clock delay.
    For(SignedDuration),
    /// The next start of a working day.
    NextMorning,
    /// 2 hours before the end of the working day on this weekday.
    BeforeCob(Weekday),
    /// The same time on the next working day.
    Dismiss,
}

/// The time when a Snooze that starts at `now` ends.
pub fn snooze_until(choice: SnoozeChoice, now: &Zoned, working: &WorkingTime) -> Timestamp {
    match choice {
        SnoozeChoice::For(duration) => now.timestamp() + duration,
        SnoozeChoice::NextMorning => next_at(now, &working.days, working.start),
        SnoozeChoice::BeforeCob(day) => next_at(
            now,
            &[day],
            working.end.saturating_sub(SignedDuration::from_hours(2)),
        ),
        SnoozeChoice::Dismiss => next_at(now, &working.days, now.time()),
    }
}

/// The first time after `now` on one of `days`, at the time of day `at`.
fn next_at(now: &Zoned, days: &[Weekday], at: Time) -> Timestamp {
    // Fifteen days is enough to find any weekday in the next week, after today.
    (0..15)
        .filter_map(|days| now.date().checked_add(jiff::Span::new().days(days)).ok())
        .filter(|date| days.contains(&date.weekday()))
        .filter_map(|date| date.to_datetime(at).to_zoned(now.time_zone().clone()).ok())
        .map(|zoned| zoned.timestamp())
        .find(|&candidate| candidate > now.timestamp())
        .unwrap_or_else(|| now.timestamp())
}

/// A duration that the user types: one or more of `<n>d`, `<n>h`, `<n>m`, for example `1h30m`.
pub fn parse_duration(text: &str) -> Option<SignedDuration> {
    let mut total = SignedDuration::ZERO;
    let mut digits = String::new();
    for c in text.trim().chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let n: i64 = digits.parse().ok()?;
        digits.clear();
        let minutes = match c {
            'd' => n.checked_mul(24 * 60)?,
            'h' => n.checked_mul(60)?,
            'm' => n,
            _ => return None,
        };
        total = total.checked_add(SignedDuration::from_mins(minutes))?;
    }
    (digits.is_empty() && total > SignedDuration::ZERO).then_some(total)
}

/// A Snooze of one pull request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Snooze {
    pub repo: String,
    pub number: u64,
    pub started_at: Timestamp,
    pub until: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnoozedItem {
    pub item: QueueItem,
    pub until: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnoozeOutcome {
    /// The Queue without the snoozed requests.
    pub visible: Vec<QueueItem>,
    /// The snoozed requests, in Queue order.
    pub snoozed: Vec<SnoozedItem>,
    /// The Snoozes that are still active. The others have ended.
    pub kept: Vec<Snooze>,
}

/// Hides the requests that have an active Snooze at `now`.
pub fn apply_snoozes(queue: Vec<QueueItem>, snoozes: &[Snooze], now: Timestamp) -> SnoozeOutcome {
    let mut outcome = SnoozeOutcome {
        visible: Vec::new(),
        snoozed: Vec::new(),
        kept: Vec::new(),
    };
    for item in queue {
        let pr = &item.pull_request;
        let active = snoozes
            .iter()
            .find(|s| s.repo == pr.repo && s.number == pr.number && now < s.until)
            // A re-request or a personal mention after the Snooze started ends it.
            .filter(|s| item.requested_at <= s.started_at)
            .filter(|s| pr.latest_mention_at.is_none_or(|at| at <= s.started_at));
        match active {
            Some(snooze) => {
                outcome.kept.push(snooze.clone());
                outcome.snoozed.push(SnoozedItem {
                    until: snooze.until,
                    item,
                });
            }
            None => outcome.visible.push(item),
        }
    }
    outcome
}
