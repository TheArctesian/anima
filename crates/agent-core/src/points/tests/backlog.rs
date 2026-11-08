//! The agent's buffer of unsent points (MET-014, K8 Section 7).

use std::collections::BTreeMap;

use chrono::TimeDelta;

use super::*;
use crate::points::{Backlog, MAX_POINTS, Outgoing, Point, SeriesKey, Summary};

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

/// The windows of `outgoing`, as numbers after [`START`].
fn windows(outgoing: Outgoing<'_>) -> Vec<i64> {
    seconds(outgoing)
        .into_iter()
        .map(|second| second / 10)
        .collect()
}

/// The times of `outgoing`, as seconds after [`START`].
fn seconds(outgoing: Outgoing<'_>) -> Vec<i64> {
    outgoing
        .points()
        .map(|point| (point.at() - at(START)).num_seconds())
        .collect()
}

/// The point `n` seconds after [`START`]. 360 windows span more than 59 minutes, so the cap
/// is seen apart from the age rule only on points closer together than windows.
fn second(n: i64) -> Point {
    point_at(at(START) + TimeDelta::seconds(n))
}

/// Sends every waiting batch at `now`, as the agent does when the hub accepts each, and gives
/// the times of the points sent, in seconds after [`START`].
fn send_all(backlog: &mut Backlog, now: DateTime<Utc>) -> Vec<i64> {
    let mut sent = Vec::new();
    loop {
        backlog.drop_stale(now);
        let outgoing = backlog.batch();
        let Some(receipt) = outgoing.receipt() else {
            return sent;
        };
        sent.extend(seconds(outgoing));
        backlog.remove_through(receipt);
    }
}

#[test]
fn met_014_the_backlog_keeps_360_points_and_drops_the_oldest_first() {
    let mut backlog = Backlog::new();
    for n in 0..400 {
        backlog.push(second(n));
    }
    let sent = send_all(&mut backlog, at(START) + TimeDelta::seconds(400));
    assert_eq!(sent.len(), MAX_POINTS);
    assert_eq!(sent.first(), Some(&40));
    assert_eq!(sent.last(), Some(&399));
}

#[test]
fn k8_7_an_hour_of_windows_is_cut_to_59_minutes_before_it_is_sent() {
    let mut backlog = Backlog::new();
    for n in 0..400 {
        backlog.push(point(n));
    }
    let sent = send_all(&mut backlog, window(399));
    assert_eq!(
        sent.first(),
        Some(&450),
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
    backlog.drop_stale(now);
    assert_eq!(windows(backlog.batch()), (2..10).collect::<Vec<_>>());
    backlog.drop_stale(now + TimeDelta::seconds(1));
    assert_eq!(windows(backlog.batch()), (3..10).collect::<Vec<_>>());
}

#[test]
fn k8_7_batches_hold_at_most_60_points_oldest_first() {
    let mut backlog = Backlog::new();
    for n in 0..150 {
        backlog.push(point(n));
    }
    for expected in [0..60, 60..120, 120..150] {
        let outgoing = backlog.batch();
        assert_eq!(windows(outgoing), expected.collect::<Vec<_>>());
        backlog.remove_through(outgoing.receipt().unwrap());
    }
    assert_eq!(backlog.batch().receipt(), None);
}

#[test]
fn met_014_a_batch_stays_until_the_caller_says_it_was_accepted() {
    let mut backlog = Backlog::new();
    for n in 0..3 {
        backlog.push(point(n));
    }
    let receipt = backlog.batch().receipt().unwrap();
    assert_eq!(
        windows(backlog.batch()),
        [0, 1, 2],
        "a failed send keeps them"
    );
    backlog.push(point(3));
    backlog.remove_through(receipt);
    assert_eq!(
        windows(backlog.batch()),
        [3],
        "a point kept meanwhile stays"
    );
}

#[test]
fn k8_6_after_too_large_the_first_points_of_a_batch_are_sent_and_removed_alone() {
    let mut backlog = Backlog::new();
    for n in 0..10 {
        backlog.push(point(n));
    }
    let smaller = backlog.batch().first(4);
    assert_eq!(windows(smaller), [0, 1, 2, 3]);
    backlog.remove_through(smaller.receipt().unwrap());
    assert_eq!(windows(backlog.batch()).first(), Some(&4));
    assert_eq!(windows(backlog.batch().first(99)).len(), 6);
}

#[test]
fn k8_7_a_batch_whose_oldest_points_were_dropped_while_it_was_sent_removes_only_its_own() {
    let mut backlog = Backlog::new();
    for n in 0..360 {
        backlog.push(second(n));
    }
    let outgoing = backlog.batch();
    let sent = seconds(outgoing);
    let receipt = outgoing.receipt().unwrap();
    assert_eq!((sent[0], sent[59]), (0, 59));
    for n in 360..370 {
        backlog.push(second(n));
    }
    backlog.remove_through(receipt);
    assert_eq!(seconds(backlog.batch()).first(), Some(&60));
}

#[test]
fn k8_4_a_point_from_a_clock_that_went_back_replaces_the_later_ones() {
    let mut backlog = Backlog::new();
    for n in [0, 1, 2, 3] {
        backlog.push(point(n));
    }
    backlog.push(point(2));
    assert_eq!(windows(backlog.batch()), [0, 1, 2]);
}

#[test]
fn met_014_a_point_kept_after_the_clock_went_back_during_a_send_survives_the_acceptance() {
    let mut backlog = Backlog::new();
    for n in 10..13 {
        backlog.push(point(n));
    }
    let receipt = backlog.batch().receipt().unwrap();
    // The clock went back while the batch of windows 10 to 12 was out.
    backlog.push(point(9));
    backlog.remove_through(receipt);
    assert_eq!(windows(backlog.batch()), [9], "the unsent point stays");
}

/// A point `n` windows after [`START`] with `count` disk series of its own.
fn disks(n: i64, count: usize) -> Point {
    let summary = Summary {
        mean: 1.0,
        max: None,
    };
    let values = (0..count)
        .map(|disk| {
            let label = format!("/n{n}d{disk}");
            let key = SeriesKey::new(Kind::DiskBytes, Some(label), None).unwrap();
            (key, summary)
        })
        .collect();
    Point::new(window(n), values)
}

#[test]
fn k8_4_a_batch_holds_up_to_256_series_and_leaves_the_point_that_makes_257() {
    let mut backlog = Backlog::new();
    backlog.push(disks(0, 128));
    backlog.push(disks(1, 128));
    backlog.push(disks(2, 1));
    let outgoing = backlog.batch();
    assert_eq!(windows(outgoing), [0, 1], "256 series fit");
    backlog.remove_through(outgoing.receipt().unwrap());
    assert_eq!(
        windows(backlog.batch()),
        [2],
        "the 257th series waits for the next batch"
    );
}

#[test]
fn k8_4_a_batch_stops_before_a_point_that_brings_it_past_256_series() {
    let mut backlog = Backlog::new();
    for n in 0..3 {
        backlog.push(disks(n, 100));
    }
    assert_eq!(
        windows(backlog.batch()),
        [0, 1],
        "a third point would bring 300 series"
    );
}
