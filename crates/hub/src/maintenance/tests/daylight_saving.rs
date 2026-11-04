//! Local start times on the days the clocks change (MW-001, MW-002).
//!
//! Dates: Auckland springs forward at 14:00 UTC on 26 September 2026 (02:00 to 03:00) and falls
//! back at 14:00 UTC on 4 April 2026 (03:00 to 02:00). New York springs forward at 07:00 UTC on
//! 8 March 2026 and falls back at 06:00 UTC on 1 November 2026. All four are Sundays locally.

use super::*;

#[test]
fn mw_001_a_start_time_inside_the_spring_forward_gap_starts_when_the_gap_ends_in_auckland() {
    // 02:30 on 27 September does not exist in Auckland; the window starts at 03:00 NZDT.
    let rule = weekly(&[Weekday::Sun], "02:30", 60);
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-09-26T13:59:59Z",
                "2026-09-26T14:00:00Z",
                "2026-09-26T14:59:59Z",
                "2026-09-26T15:00:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_001_a_start_time_inside_the_spring_forward_gap_starts_when_the_gap_ends_in_new_york() {
    let rule = weekly(&[Weekday::Sun], "02:30", 30);
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &[
                "2026-03-08T06:59:59Z",
                "2026-03-08T07:00:00Z",
                "2026-03-08T07:29:59Z",
                "2026-03-08T07:30:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_001_a_start_time_that_happens_twice_uses_the_earlier_one_in_auckland() {
    // 02:30 on 5 April happens at 13:30 UTC (NZDT) and again at 14:30 UTC (NZST).
    let rule = weekly(&[Weekday::Sun], "02:30", 60);
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-04-04T13:29:59Z",
                "2026-04-04T13:30:00Z",
                "2026-04-04T14:29:59Z",
                "2026-04-04T14:30:00Z",
                "2026-04-04T15:00:00Z",
            ]
        ),
        [false, true, true, false, false]
    );
}

#[test]
fn mw_001_a_start_time_that_happens_twice_uses_the_earlier_one_in_new_york() {
    // 01:30 on 1 November happens at 05:30 UTC (EDT) and again at 06:30 UTC (EST).
    let rule = weekly(&[Weekday::Sun], "01:30", 60);
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &[
                "2026-11-01T05:29:59Z",
                "2026-11-01T05:30:00Z",
                "2026-11-01T06:29:59Z",
                "2026-11-01T06:30:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_002_a_window_lasts_its_duration_in_elapsed_time_across_the_spring_forward_gap() {
    // Saturday 23:30 NZST for three hours: 11:30 UTC to 14:30 UTC, which reads 03:30 NZDT.
    let rule = weekly(&[Weekday::Sat], "23:30", 180);
    assert_eq!(
        states(
            &rule,
            AUCKLAND,
            &[
                "2026-09-26T11:29:59Z",
                "2026-09-26T11:30:00Z",
                "2026-09-26T14:29:59Z",
                "2026-09-26T14:30:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_002_a_window_lasts_its_duration_in_elapsed_time_across_the_fall_back_repeat() {
    // Saturday 22:00 EDT for four hours: 02:00 UTC to 06:00 UTC, which reads 01:00 EST.
    let rule = weekly(&[Weekday::Sat], "22:00", 240);
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &[
                "2026-11-01T01:59:59Z",
                "2026-11-01T02:00:00Z",
                "2026-11-01T05:59:59Z",
                "2026-11-01T06:00:00Z",
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_001_a_day_of_month_window_in_the_gap_also_starts_when_the_gap_ends() {
    let rule = monthly(&[8], "02:15", 60);
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &[
                "2026-03-08T06:59:59Z",
                "2026-03-08T07:00:00Z",
                "2026-03-08T07:59:59Z",
                "2026-03-08T08:00:00Z"
            ]
        ),
        [false, true, true, false]
    );
}

#[test]
fn mw_002_a_seven_day_window_that_crosses_a_short_day_is_found_eight_local_days_later() {
    // 5 March 23:30 EST for 168 elapsed hours; the clocks spring forward on 8 March, so the
    // window ends at 00:30 EDT on 13 March, which is eight local days after it began.
    let rule = monthly(&[5], "23:30", 10_080);
    assert_eq!(
        states(
            &rule,
            NEW_YORK,
            &["2026-03-13T04:15:00Z", "2026-03-13T04:30:00Z"]
        ),
        [true, false]
    );
}
