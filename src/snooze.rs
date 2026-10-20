//! Snooze: when a Snooze ends.

use jiff::civil::{Time, Weekday};
use jiff::{SignedDuration, Timestamp, Zoned};

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
        SnoozeChoice::NextMorning => next_working(now, working, |_| working.start),
        _ => now.timestamp(),
    }
}

/// The first time after `now`, on a working day, at the time of day that `at` gives.
fn next_working(now: &Zoned, working: &WorkingTime, at: impl Fn(Weekday) -> Time) -> Timestamp {
    // Two weeks is enough to find any working day.
    (0..15)
        .filter_map(|days| now.date().checked_add(jiff::Span::new().days(days)).ok())
        .filter(|date| working.days.contains(&date.weekday()))
        .filter_map(|date| {
            date.to_datetime(at(date.weekday()))
                .to_zoned(now.time_zone().clone())
                .ok()
        })
        .map(|zoned| zoned.timestamp())
        .find(|&candidate| candidate > now.timestamp())
        .unwrap_or_else(|| now.timestamp())
}
