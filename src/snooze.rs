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
pub fn snooze_until(choice: SnoozeChoice, now: &Zoned, _working: &WorkingTime) -> Timestamp {
    match choice {
        SnoozeChoice::For(duration) => now.timestamp() + duration,
        _ => now.timestamp(),
    }
}
