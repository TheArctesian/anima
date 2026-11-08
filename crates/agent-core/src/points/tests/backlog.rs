//! The agent's buffer of unsent points (MET-014, K8 Section 7).

use std::collections::BTreeMap;

use chrono::TimeDelta;

use super::*;
use crate::points::{BATCH_POINTS, Backlog, MAX_POINTS, MAX_SERIES, Point, SeriesKey, Summary};

const START: &str = "2026-11-08T10:00:00Z";

/// The point `n` windows after [`START`], with one CPU value.
fn point(n: i64) -> Point {
    point_at(window(n))
}

fn point_at(time: DateTime<Utc>) -> Point {
    let cpu = SeriesKey::new(Kind::CpuPercent, None, None).unwrap();
    let summary = Summary {
        mean: 1.0,
        max: Some(1.0),
    };
    Point::new(time, BTreeMap::from([(cpu, summary)]))
}

fn window(n: i64) -> DateTime<Utc> {
    at(START) + TimeDelta::seconds(10 * n)
}

/// The windows of `points`, as numbers after [`START`].
fn windows(points: &[Point]) -> Vec<i64> {
    seconds(points)
        .into_iter()
        .map(|second| second / 10)
        .collect()
}

/// The times of `points`, as seconds after [`START`].
fn seconds(points: &[Point]) -> Vec<i64> {
    points
        .iter()
        .map(|point| (point.at() - at(START)).num_seconds())
        .collect()
}

/// The point `n` seconds after [`START`]. 360 windows span more than 59 minutes, so the cap
/// is seen apart from the age rule only on points closer together than windows.
fn second(n: i64) -> Point {
    point_at(at(START) + TimeDelta::seconds(n))
}

#[test]
fn met_014_the_backlog_keeps_360_points_and_drops_the_oldest_first() {
    let mut backlog = Backlog::new();
    for n in 0..400 {
        backlog.push(second(n));
    }
    let now = at(START) + TimeDelta::seconds(400);
    let mut kept = Vec::new();
    loop {
        let batch = seconds(backlog.batch(now));
        let Some(&newest) = batch.last() else { break };
        backlog.remove_through(at(START) + TimeDelta::seconds(newest));
        kept.extend(batch);
    }
    assert_eq!(kept.len(), MAX_POINTS);
    assert_eq!(kept.first(), Some(&40));
    assert_eq!(kept.last(), Some(&399));
}

#[test]
fn k8_7_an_hour_of_windows_is_cut_to_59_minutes_before_it_is_sent() {
    let mut backlog = Backlog::new();
    for n in 0..400 {
        backlog.push(point(n));
    }
    let sent = windows(backlog.batch(window(399)));
    assert_eq!(
        sent.first(),
        Some(&45),
        "window 45 is exactly 59 minutes before window 399"
    );
}

#[test]
fn k8_7_points_older_than_59_minutes_are_dropped_before_a_send() {
    let mut backlog = Backlog::new();
    for n in 0..10 {
        backlog.push(point(n));
    }
    // 59 minutes after window 2: window 2 is exactly 59 minutes old and stays.
    let now = window(2) + TimeDelta::minutes(59);
    assert_eq!(windows(backlog.batch(now)), (2..10).collect::<Vec<_>>());
    let later = now + TimeDelta::seconds(1);
    assert_eq!(windows(backlog.batch(later)), (3..10).collect::<Vec<_>>());
}

#[test]
fn k8_7_batches_hold_at_most_60_points_oldest_first() {
    let mut backlog = Backlog::new();
    for n in 0..150 {
        backlog.push(point(n));
    }
    let now = window(150);
    let first = windows(backlog.batch(now));
    assert_eq!(first, (0..60).collect::<Vec<_>>());
    assert_eq!(BATCH_POINTS, 60);
    backlog.remove_through(window(59));
    assert_eq!(windows(backlog.batch(now)), (60..120).collect::<Vec<_>>());
    backlog.remove_through(window(119));
    assert_eq!(windows(backlog.batch(now)), (120..150).collect::<Vec<_>>());
    backlog.remove_through(window(149));
    assert_eq!(backlog.batch(now), []);
}

#[test]
fn met_014_a_batch_stays_until_the_caller_says_it_was_accepted() {
    let mut backlog = Backlog::new();
    for n in 0..3 {
        backlog.push(point(n));
    }
    let now = window(3);
    assert_eq!(windows(backlog.batch(now)), [0, 1, 2]);
    assert_eq!(
        windows(backlog.batch(now)),
        [0, 1, 2],
        "a failed send keeps them"
    );
    backlog.push(point(3));
    backlog.remove_through(window(2));
    assert_eq!(
        windows(backlog.batch(now)),
        [3],
        "a point kept meanwhile stays"
    );
}

#[test]
fn k8_7_a_batch_whose_oldest_points_were_dropped_while_it_was_sent_removes_only_its_own() {
    let mut backlog = Backlog::new();
    for n in 0..360 {
        backlog.push(second(n));
    }
    let now = at(START) + TimeDelta::seconds(360);
    let sent = seconds(backlog.batch(now));
    assert_eq!((sent[0], sent[59]), (0, 59));
    for n in 360..370 {
        backlog.push(second(n));
    }
    backlog.remove_through(at(START) + TimeDelta::seconds(59));
    assert_eq!(seconds(backlog.batch(now)).first(), Some(&60));
}

#[test]
fn k8_4_a_point_from_a_clock_that_went_back_replaces_the_later_ones() {
    let mut backlog = Backlog::new();
    for n in [0, 1, 2, 3] {
        backlog.push(point(n));
    }
    backlog.push(point(2));
    assert_eq!(windows(backlog.batch(window(4))), [0, 1, 2]);
}

#[test]
fn k8_4_a_batch_holds_at_most_256_series_between_its_points() {
    let mut backlog = Backlog::new();
    let summary = Summary {
        mean: 1.0,
        max: None,
    };
    for n in 0..3 {
        let values = (0..100)
            .map(|disk| {
                let label = format!("/n{n}d{disk}");
                let key = SeriesKey::new(Kind::DiskBytes, Some(label), None).unwrap();
                (key, summary)
            })
            .collect();
        backlog.push(Point::new(window(n), values));
    }
    let batch = backlog.batch(window(3));
    assert_eq!(
        windows(batch),
        [0, 1],
        "a third point would bring 300 series"
    );
    const { assert!(MAX_SERIES >= 200) };
}
