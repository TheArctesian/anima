//! When an account syncs next, and which due accounts the scheduler runs (SRC-021, K-E1
//! Section 10). Pure: time, the interval and the limits are parameters.

use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::MIN_POLL_INTERVAL_SECONDS;

use crate::ids::SourceAccountId;

/// The longest wait after failures.
pub const MAX_BACKOFF: TimeDelta = TimeDelta::hours(6);

/// The poll interval of a source: what it declares, never under the 15-minute floor (SRC-021).
#[must_use]
pub fn interval(declared_seconds: u32) -> TimeDelta {
    TimeDelta::seconds(i64::from(declared_seconds.max(MIN_POLL_INTERVAL_SECONDS)))
}

/// The next sync after a run that ended at `now`. `failure_count` is the count after the run:
/// 0 after a success, which waits one interval; after a failure the wait doubles with each
/// failure in a row, up to [`MAX_BACKOFF`]. A rate limit waits at least until `until`.
#[must_use]
pub fn next(
    now: DateTime<Utc>,
    interval: TimeDelta,
    failure_count: u32,
    rate_limited_until: Option<DateTime<Utc>>,
) -> DateTime<Utc> {
    let wait = if failure_count == 0 {
        interval
    } else {
        // 2^16 intervals of at least 15 minutes is far past the cap, so a longer run of
        // failures cannot overflow.
        let factor = 1_i32 << failure_count.min(16);
        (interval * factor).min(MAX_BACKOFF).max(interval)
    };
    let next = now + wait;
    rate_limited_until.map_or(next, |until| next.max(until))
}

/// An account the store found due.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    pub id: SourceAccountId,
    pub source: String,
}

/// The due accounts to start now: those whose source has a consumer that is on, oldest first
/// as given, at most `room` of them. An account with no consumer on is left as it is, so its
/// `next_sync_at` is not touched (EXT-002).
#[must_use]
pub fn runnable(
    due: &[Due],
    consumer_on: impl Fn(&str) -> bool,
    room: usize,
) -> Vec<SourceAccountId> {
    due.iter()
        .filter(|account| consumer_on(&account.source))
        .take(room)
        .map(|account| account.id)
        .collect()
}
