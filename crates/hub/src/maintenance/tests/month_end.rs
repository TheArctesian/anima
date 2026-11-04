//! Days of the month, including the last day (MW-004).

use super::*;

#[test]
fn mw_004_day_31_does_not_happen_in_a_30_day_month() {
    let rule = monthly(&[31], "00:00", 1440);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-04-30T12:00:00Z", // April has 30 days
                "2026-05-01T12:00:00Z",
                "2026-05-31T12:00:00Z",
            ]
        ),
        [false, false, true]
    );
}

#[test]
fn mw_004_minus_one_is_the_last_day_of_each_month() {
    let rule = monthly(&[-1], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-04-29T00:30:00Z",
                "2026-04-30T00:30:00Z", // 30-day month
                "2026-05-30T00:30:00Z",
                "2026-05-31T00:30:00Z", // 31-day month
                "2026-12-31T00:30:00Z", // year end
            ]
        ),
        [false, true, false, true, true]
    );
}

#[test]
fn mw_004_minus_one_is_the_28th_of_february_in_a_common_year() {
    let rule = monthly(&[-1], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2027-02-27T00:30:00Z",
                "2027-02-28T00:30:00Z",
                "2027-03-01T00:30:00Z"
            ]
        ),
        [false, true, false]
    );
}

#[test]
fn mw_004_minus_one_is_the_29th_of_february_in_a_leap_year() {
    let rule = monthly(&[-1], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2028-02-28T00:30:00Z",
                "2028-02-29T00:30:00Z",
                "2028-03-01T00:30:00Z"
            ]
        ),
        [false, true, false]
    );
}

#[test]
fn mw_004_day_29_exists_in_february_only_in_a_leap_year() {
    let rule = monthly(&[29], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2027-02-28T00:30:00Z",
                "2027-03-01T00:30:00Z",
                "2028-02-29T00:30:00Z"
            ]
        ),
        [false, false, true]
    );
}

#[test]
fn mw_002_a_last_day_window_can_run_into_the_next_month() {
    let rule = monthly(&[-1], "23:30", 90);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-12-31T23:45:00Z",
                "2027-01-01T00:29:59Z",
                "2027-01-01T01:00:00Z"
            ]
        ),
        [true, true, false]
    );
}

#[test]
fn mw_001_the_day_of_the_month_is_the_local_one() {
    // 00:30 NZDT on 15 November 2026 is 11:30 UTC on the 14th.
    let rule = monthly(&[15], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &["2026-11-14T11:30:00Z", "2026-11-15T11:30:00Z"]
        ),
        [true, false]
    );
}

#[test]
fn mw_004_several_days_of_the_month_may_be_listed_together() {
    let rule = monthly(&[1, 15, -1], "00:00", 60);
    assert_eq!(
        states(
            &rule,
            UTC,
            &[
                "2026-11-01T00:30:00Z",
                "2026-11-02T00:30:00Z",
                "2026-11-15T00:30:00Z",
                "2026-11-30T00:30:00Z"
            ]
        ),
        [true, false, true, true]
    );
}
