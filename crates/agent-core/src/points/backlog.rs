//! The points the agent has not yet seen accepted (MET-014, K8 Section 7).
//!
//! Pure: the time is a parameter, and the retry pace belongs to the caller (CQ-021).

use std::collections::{BTreeSet, VecDeque};

use chrono::{DateTime, TimeDelta, Utc};

use super::{MAX_SERIES, Point};

/// The most points the agent keeps, and the most one batch holds: one hour (K8 Sections 4, 7).
pub const MAX_POINTS: usize = 360;

/// The most points one batch the agent sends holds (K8 Section 7).
pub const BATCH_POINTS: usize = 60;

/// A point older than this before a send is dropped, so the hub never sees one past its
/// 1-hour bound (K8 Sections 6, 7).
pub const MAX_AGE: TimeDelta = TimeDelta::minutes(59);

/// The points waiting to be sent, oldest first, with `at` strictly increasing.
#[derive(Debug, Default)]
pub struct Backlog {
    points: VecDeque<Point>,
}

impl Backlog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps `point` as the newest. A kept point at or after its `at` is dropped first (the
    /// clock went back), so a batch's times stay strictly increasing; past [`MAX_POINTS`] the
    /// oldest are dropped.
    pub fn push(&mut self, point: Point) {
        while self
            .points
            .back()
            .is_some_and(|newest| newest.at() >= point.at())
        {
            self.points.pop_back();
        }
        self.points.push_back(point);
        while self.points.len() > MAX_POINTS {
            self.points.pop_front();
        }
    }

    /// Drops the points older than [`MAX_AGE`] before `now`, then gives the oldest points to
    /// send next: at most [`BATCH_POINTS`], with at most [`MAX_SERIES`] series between them.
    /// Empty when none wait. The points stay until [`Backlog::remove_through`]; a caller that
    /// was told the body is too large may send the first ones of this slice only.
    pub fn batch(&mut self, now: DateTime<Utc>) -> &[Point] {
        while self
            .points
            .front()
            .is_some_and(|oldest| now - oldest.at() > MAX_AGE)
        {
            self.points.pop_front();
        }
        let mut series = BTreeSet::new();
        let mut count = 0;
        for point in self.points.iter().take(BATCH_POINTS) {
            series.extend(point.values().keys());
            if count > 0 && series.len() > MAX_SERIES {
                break;
            }
            count += 1;
        }
        &self.points.make_contiguous()[..count]
    }

    /// Removes the points at or before `through`: the `at` of the newest point of a batch the
    /// hub accepted, or refused for good (K8 Section 6). Points kept since then stay.
    pub fn remove_through(&mut self, through: DateTime<Utc>) {
        while self
            .points
            .front()
            .is_some_and(|oldest| oldest.at() <= through)
        {
            self.points.pop_front();
        }
    }
}
