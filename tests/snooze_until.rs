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

#[test]
fn next_morning_on_a_working_afternoon_is_the_start_of_the_next_working_day() {
    assert_eq!(
        until(SnoozeChoice::NextMorning, "2026-10-21T14:00"),
        local("2026-10-22T09:00").timestamp()
    );
}

#[test]
fn next_morning_on_friday_is_monday_morning() {
    assert_eq!(
        until(SnoozeChoice::NextMorning, "2026-10-23T14:00"),
        local("2026-10-26T09:00").timestamp()
    );
}

#[test]
fn next_morning_on_saturday_is_monday_morning() {
    assert_eq!(
        until(SnoozeChoice::NextMorning, "2026-10-24T10:00"),
        local("2026-10-26T09:00").timestamp()
    );
}

#[test]
fn next_morning_before_working_hours_is_the_same_morning() {
    assert_eq!(
        until(SnoozeChoice::NextMorning, "2026-10-21T07:00"),
        local("2026-10-21T09:00").timestamp()
    );
}

use jiff::civil::Weekday;

#[test]
fn before_cob_friday_on_wednesday_is_friday_two_hours_before_close_of_business() {
    assert_eq!(
        until(SnoozeChoice::BeforeCob(Weekday::Friday), "2026-10-21T10:00"),
        local("2026-10-23T15:30").timestamp()
    );
}

#[test]
fn before_cob_friday_on_friday_morning_is_the_same_day() {
    assert_eq!(
        until(SnoozeChoice::BeforeCob(Weekday::Friday), "2026-10-23T10:00"),
        local("2026-10-23T15:30").timestamp()
    );
}

#[test]
fn before_cob_friday_after_that_time_on_friday_is_next_friday() {
    assert_eq!(
        until(SnoozeChoice::BeforeCob(Weekday::Friday), "2026-10-23T16:00"),
        local("2026-10-30T15:30").timestamp()
    );
}
