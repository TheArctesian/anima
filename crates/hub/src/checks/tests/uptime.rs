//! Aggregate buckets and the uptime ratio (DB-008, CHK-011).

use chrono::{DateTime, Utc};

use crate::checks::{
    UptimeWindow,
    uptime::{Resolution, Tally, ratio},
};

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

#[test]
fn db_008_a_beat_falls_in_the_utc_minute_hour_and_day_that_hold_it() {
    let beat = at("2026-11-05T09:59:59.999+13:00");
    assert_eq!(Resolution::Minute.bucket(beat), at("2026-11-04T20:59:00Z"));
    assert_eq!(Resolution::Hour.bucket(beat), at("2026-11-04T20:00:00Z"));
    assert_eq!(Resolution::Day.bucket(beat), at("2026-11-04T00:00:00Z"));
}

#[test]
fn db_008_a_beat_on_a_boundary_starts_its_bucket() {
    let midnight = at("2026-11-05T00:00:00Z");
    for resolution in Resolution::ALL {
        assert_eq!(resolution.bucket(midnight), midnight);
    }
}

#[test]
fn chk_011_each_window_reads_the_table_kept_long_enough() {
    let resolutions = [
        UptimeWindow::Day,
        UptimeWindow::Week,
        UptimeWindow::Month,
        UptimeWindow::Year,
    ]
    .map(UptimeWindow::resolution);
    assert_eq!(
        resolutions,
        [
            Resolution::Minute,
            Resolution::Hour,
            Resolution::Hour,
            Resolution::Day
        ]
    );
}

#[test]
fn chk_011_windows_end_now_and_reach_back_their_length() {
    let now = at("2026-11-04T09:00:00Z");
    assert_eq!(UptimeWindow::Day.start(now), at("2026-11-03T09:00:00Z"));
    assert_eq!(UptimeWindow::Week.start(now), at("2026-10-28T09:00:00Z"));
    assert_eq!(UptimeWindow::Month.start(now), at("2026-10-05T09:00:00Z"));
    assert_eq!(UptimeWindow::Year.start(now), at("2025-11-04T09:00:00Z"));
}

#[test]
fn chk_011_uptime_counts_degraded_as_up_and_ignores_the_rest() {
    let tally = Tally {
        up: 6,
        down: 2,
        degraded: 2,
    };
    assert_eq!(ratio(tally), Some(0.8));
    assert_eq!(ratio(Tally { down: 0, ..tally }), Some(1.0));
    assert_eq!(
        ratio(Tally {
            up: 0,
            degraded: 0,
            down: 3
        }),
        Some(0.0)
    );
}

#[test]
fn chk_011_no_counted_beats_gives_no_uptime() {
    assert_eq!(ratio(Tally::default()), None);
}
