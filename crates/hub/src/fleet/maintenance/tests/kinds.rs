//! Each kind of rule, and windows that run past midnight (MW-002, MW-004).

use super::*;
use crate::{fleet::maintenance::model::Window, ids::WindowId};

#[test]
fn mw_004_a_manual_rule_is_never_active_on_its_own() {
    assert!(!is_active(
        &WindowRule::Manual,
        UTC,
        at("2026-11-04T09:00:00Z")
    ));
}

#[test]
fn mw_004_a_stored_duration_beyond_seven_days_is_clamped_and_stays_fast() {
    let rule = monthly(&[2], "00:00", u32::MAX);
    let started = std::time::Instant::now();
    assert!(is_active(&rule, UTC, at("2026-11-08T23:00:00Z")));
    assert!(!is_active(&rule, UTC, at("2026-11-12T00:00:00Z")));
    assert!(
        started.elapsed() < std::time::Duration::from_millis(50),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn mw_004_a_manual_window_follows_its_stored_switch() {
    let window = |active_manual| Window {
        id: WindowId::from(uuid::Uuid::nil()),
        name: "w".into(),
        timezone: UTC,
        rule: WindowRule::Manual,
        active_manual,
        targets: vec![],
        created_at: at("2026-11-04T09:00:00Z"),
        updated_at: at("2026-11-04T09:00:00Z"),
    };
    let now = at("2026-11-04T09:00:00Z");
    assert!(window(true).is_active(now));
    assert!(!window(false).is_active(now));
}

#[test]
fn mw_004_a_single_window_starts_inclusive_and_ends_exclusive() {
    let rule = WindowRule::Single {
        starts_at: at("2026-11-04T09:00:00Z"),
        ends_at: at("2026-11-04T10:00:00Z"),
    };
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-11-04T08:59:59Z",
                "2026-11-04T09:00:00Z",
                "2026-11-04T09:59:59Z",
                "2026-11-04T10:00:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_004_an_interval_window_repeats_every_n_days_from_its_first_start() {
    let rule = WindowRule::RecurringInterval {
        first_start_at: at("2026-11-01T00:00:00Z"),
        every_days: 3,
        duration_minutes: 60,
    };
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-10-29T00:30:00Z", // a due day, but before the first start
                "2026-11-01T00:00:00Z", // first start, inclusive
                "2026-11-01T00:59:59Z",
                "2026-11-01T01:00:00Z", // end, exclusive
                "2026-11-02T00:30:00Z", // not a due day
                "2026-11-04T00:30:00Z", // 3 days later
                "2026-11-07T00:30:00Z", // 6 days later
                "2026-11-08T00:30:00Z", // not a due day
            ]
        ),
        [false, true, true, false, false, true, true, false]
    );
}

#[test]
fn mw_001_an_interval_window_keeps_its_local_time_across_a_daylight_saving_change() {
    // Daily at 09:00 NZST (21:00 UTC the day before). Auckland moves to NZDT (+13) at 14:00 UTC
    // on 26 September 2026, so later starts are at 20:00 UTC, not 21:00.
    let rule = WindowRule::RecurringInterval {
        first_start_at: at("2026-09-24T21:00:00Z"),
        every_days: 1,
        duration_minutes: 60,
    };
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-09-26T20:30:00Z", // 09:30 NZDT
                "2026-09-26T21:30:00Z", // 10:30 NZDT
                "2026-09-27T20:30:00Z",
                "2026-09-27T21:30:00Z",
            ]
        ),
        [true, false, true, false]
    );
}

#[test]
fn mw_001_an_interval_window_starts_at_the_instant_given_even_when_that_local_time_repeats() {
    // 01:30 on 1 November 2026 happens twice in New York; the operator gave the second one.
    let rule = WindowRule::RecurringInterval {
        first_start_at: at("2026-11-01T06:30:00Z"),
        every_days: 1,
        duration_minutes: 60,
    };
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &[
                "2026-11-01T05:45:00Z",
                "2026-11-01T06:45:00Z",
                "2026-11-02T06:45:00Z"
            ]
        ),
        [false, true, true]
    );
}

#[test]
fn mw_002_a_window_that_starts_at_23_30_for_90_minutes_runs_into_the_next_day() {
    let every_day = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let rule = weekly(&every_day, "23:30", 90);
    // 4 November 2026 in Auckland is NZDT (+13): 23:30 local is 10:30 UTC.
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-11-04T10:29:59Z",
                "2026-11-04T10:30:00Z",
                "2026-11-04T11:30:00Z", // 00:30 on the 5th
                "2026-11-04T11:59:59Z",
                "2026-11-04T12:00:00Z", // 01:00, exclusive
            ]
        ),
        [false, true, true, true, false]
    );
}

#[test]
fn mw_002_a_window_past_midnight_belongs_to_the_day_it_started_on() {
    // Fridays 23:00 for two hours (UTC). Saturday 00:30 is inside it; Saturday 23:30 is not,
    // because Saturday is not listed.
    let rule = weekly(&[Weekday::Fri], "23:00", 120);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-11-06T22:59:59Z",
                "2026-11-06T23:00:00Z",
                "2026-11-07T00:30:00Z",
                "2026-11-07T00:59:59Z",
                "2026-11-07T01:00:00Z",
                "2026-11-07T23:30:00Z",
            ]
        ),
        [false, true, true, true, false, false]
    );
}

#[test]
fn mw_002_a_window_can_run_for_several_days() {
    // Fridays 22:00 for three days (4320 minutes) ends Monday 22:00.
    let rule = weekly(&[Weekday::Fri], "22:00", 4320);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-11-08T12:00:00Z", // Sunday
                "2026-11-09T21:59:59Z", // Monday, last second
                "2026-11-09T22:00:00Z",
            ]
        ),
        [true, true, false]
    );
}

#[test]
fn mw_002_a_seven_day_window_that_started_seven_days_ago_is_still_found() {
    // Tuesdays 23:59: the occurrence that started on 3 November lasts until 23:59 on the 10th,
    // so on the 10th its start is seven days back. One minute shorter and it has just ended.
    let week = weekly(&[Weekday::Tue], "23:59", 10_080);
    let almost_week = weekly(&[Weekday::Tue], "23:59", 10_079);
    assert!(is_active(&week, UTC, at("2026-11-10T23:58:00Z")));
    assert!(is_active(&almost_week, UTC, at("2026-11-10T23:57:00Z")));
    assert!(!is_active(&almost_week, UTC, at("2026-11-10T23:58:00Z")));
}

#[test]
fn mw_001_the_weekday_is_the_local_one_not_the_utc_one() {
    // Sundays 03:00 for an hour in Auckland. 03:00 NZDT on Sunday 8 November 2026 is
    // Saturday 7 November 14:00 UTC.
    let rule = weekly(&[Weekday::Sun], "03:00", 60);
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-11-07T13:59:59Z",
                "2026-11-07T14:00:00Z",
                "2026-11-07T14:59:59Z",
                "2026-11-07T15:00:00Z"
            ]
        ),
        [false, true, true, false]
    );
}
