use jiff::{SignedDuration, Zoned};
use review_queue::snooze::{WorkingTime, working_duration};

/// A local time in Melbourne. 2026-10-21 is a Wednesday. The default working time is
/// 09:00–17:30, Monday to Friday.
fn local(s: &str) -> Zoned {
    format!("{s}[Australia/Melbourne]").parse().unwrap()
}

fn hours(from: &str, to: &str) -> f64 {
    let d = working_duration(local(from).timestamp(), &local(to), &WorkingTime::default());
    d.as_secs_f64() / 3600.0
}

#[test]
fn time_inside_one_working_day_counts_in_full() {
    assert_eq!(hours("2026-10-21T10:00", "2026-10-21T13:30"), 3.5);
}

#[test]
fn time_before_and_after_working_hours_does_not_count() {
    assert_eq!(hours("2026-10-21T07:00", "2026-10-21T10:00"), 1.0);
    assert_eq!(hours("2026-10-21T16:30", "2026-10-21T22:00"), 1.0);
}

#[test]
fn a_night_counts_only_the_working_hours_on_each_side() {
    // 16:30–17:30 on Wednesday and 09:00–10:00 on Thursday.
    assert_eq!(hours("2026-10-21T16:30", "2026-10-22T10:00"), 2.0);
}

#[test]
fn a_weekend_does_not_count() {
    // 16:30–17:30 on Friday and 09:00–10:00 on Monday.
    assert_eq!(hours("2026-10-23T16:30", "2026-10-26T10:00"), 2.0);
    assert_eq!(hours("2026-10-24T10:00", "2026-10-25T15:00"), 0.0);
}

#[test]
fn whole_working_days_count_8_and_a_half_hours_each() {
    assert_eq!(hours("2026-10-19T09:00", "2026-10-23T17:30"), 5.0 * 8.5);
}

#[test]
fn a_start_after_the_end_is_zero() {
    assert_eq!(
        working_duration(
            local("2026-10-21T12:00").timestamp(),
            &local("2026-10-21T10:00"),
            &WorkingTime::default()
        ),
        SignedDuration::ZERO
    );
}
