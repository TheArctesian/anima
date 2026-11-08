//! The points the agent has not yet seen accepted (MET-014, K8 Section 7).
//!
//! Pure: the time is a parameter, and the retry pace belongs to the caller (CQ-021).

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};

use super::{MAX_SERIES, Point};

/// The most points the agent keeps, and the most one batch holds: one hour (K8 Sections 4, 7).
pub const MAX_POINTS: usize = 360;

/// The most points one batch the agent sends holds (K8 Section 7).
pub const BATCH_POINTS: usize = 60;

/// A point older than this before a send is dropped, so the hub never sees one past its
/// 1-hour bound (K8 Sections 6, 7).
pub const MAX_AGE: TimeDelta = TimeDelta::minutes(59);

/// The points waiting to be sent, oldest first, with `at` strictly increasing. Each has a
/// sequence number given when it was kept, which only grows, so a [`Receipt`] removes exactly
/// the points that were sent, even when the clock went back during the send.
#[derive(Debug, Default)]
pub struct Backlog {
    kept: Vec<Kept>,
    next: u64,
}

/// A point and its sequence number.
#[derive(Debug)]
struct Kept {
    sequence: u64,
    point: Point,
}

/// The oldest points to send next, from [`Backlog::batch`].
#[derive(Clone, Copy, Debug)]
pub struct Outgoing<'a> {
    kept: &'a [Kept],
}

/// What removes the points of one batch once the hub accepted it or refused it for good: the
/// sequence number of its newest point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt(u64);

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
            .kept
            .last()
            .is_some_and(|newest| newest.point.at() >= point.at())
        {
            self.kept.pop();
        }
        self.kept.push(Kept {
            sequence: self.next,
            point,
        });
        self.next += 1;
        let over = self.kept.len().saturating_sub(MAX_POINTS);
        self.kept.drain(..over);
    }

    /// Drops the points older than [`MAX_AGE`] before `now`. Call it before each
    /// [`Backlog::batch`] (K8 Section 7).
    pub fn drop_stale(&mut self, now: DateTime<Utc>) {
        let stale = self
            .kept
            .iter()
            .take_while(|kept| now - kept.point.at() > MAX_AGE)
            .count();
        self.kept.drain(..stale);
    }

    /// The oldest points to send next: at most [`BATCH_POINTS`], with at most [`MAX_SERIES`]
    /// series between them. Empty when none wait. They stay until [`Backlog::remove_through`].
    #[must_use]
    pub fn batch(&self) -> Outgoing<'_> {
        let mut series = BTreeSet::new();
        let mut count = 0;
        for kept in self.kept.iter().take(BATCH_POINTS) {
            series.extend(kept.point.values().keys());
            if count > 0 && series.len() > MAX_SERIES {
                break;
            }
            count += 1;
        }
        Outgoing {
            kept: &self.kept[..count],
        }
    }

    /// Removes the points of the batch `receipt` came from, and any kept before them: the hub
    /// accepted that batch, or refused it for good (K8 Section 6). Points kept after the batch
    /// was made stay, whatever their time.
    pub fn remove_through(&mut self, receipt: Receipt) {
        let sent = self
            .kept
            .iter()
            .take_while(|kept| kept.sequence <= receipt.0)
            .count();
        self.kept.drain(..sent);
    }
}

impl<'a> Outgoing<'a> {
    /// The points, oldest first.
    pub fn points(self) -> impl ExactSizeIterator<Item = &'a Point> {
        self.kept.iter().map(|kept| &kept.point)
    }

    /// The receipt of these points; `None` when there are none.
    #[must_use]
    pub fn receipt(self) -> Option<Receipt> {
        self.kept.last().map(|kept| Receipt(kept.sequence))
    }

    /// The first `count` points only, for a smaller batch after the hub said the body was too
    /// large (K8 Section 6).
    #[must_use]
    pub fn first(self, count: usize) -> Self {
        Self {
            kept: &self.kept[..count.min(self.kept.len())],
        }
    }
}
