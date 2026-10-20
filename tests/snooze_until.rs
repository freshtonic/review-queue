use jiff::{SignedDuration, Timestamp, Zoned};
use review_queue::snooze::{SnoozeChoice, WorkingTime, snooze_until};

/// A local time in Melbourne. 2026-10-21 is a Wednesday.
fn local(s: &str) -> Zoned {
    format!("{s}[Australia/Melbourne]").parse().unwrap()
}

fn until(choice: SnoozeChoice, now: &str) -> Timestamp {
    snooze_until(choice, &local(now), &WorkingTime::default())
}

#[test]
fn a_duration_ends_that_much_wall_clock_time_after_now() {
    assert_eq!(
        until(SnoozeChoice::For(SignedDuration::from_hours(3)), "2026-10-21T14:10"),
        local("2026-10-21T17:10").timestamp()
    );
}
