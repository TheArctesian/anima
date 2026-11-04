//! Uptime windows, the aggregate buckets a heartbeat falls in, and the uptime ratio
//! (DB-008, CHK-011). Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, DurationRound, TimeDelta, Utc};

/// The period an uptime figure covers (the API's `UptimeWindow`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UptimeWindow {
    Day,
    Week,
    Month,
    Year,
}

/// One of the three aggregate tables (DB-008).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Resolution {
    Minute,
    Hour,
    Day,
}

impl Resolution {
    pub(super) const ALL: [Self; 3] = [Self::Minute, Self::Hour, Self::Day];

    const fn step(self) -> TimeDelta {
        match self {
            Self::Minute => TimeDelta::minutes(1),
            Self::Hour => TimeDelta::hours(1),
            Self::Day => TimeDelta::days(1),
        }
    }

    /// The UTC start of the bucket that holds `at`.
    #[must_use]
    pub(super) fn bucket(self, at: DateTime<Utc>) -> DateTime<Utc> {
        // Truncation to a whole minute, hour or UTC day only fails out of chrono's range.
        at.duration_trunc(self.step()).unwrap_or(at)
    }
}

impl UptimeWindow {
    const fn length(self) -> TimeDelta {
        match self {
            Self::Day => TimeDelta::days(1),
            Self::Week => TimeDelta::days(7),
            Self::Month => TimeDelta::days(30),
            Self::Year => TimeDelta::days(365),
        }
    }

    /// The finest table kept long enough to cover the window (DB-009).
    #[must_use]
    pub(super) const fn resolution(self) -> Resolution {
        match self {
            Self::Day => Resolution::Minute,
            Self::Week | Self::Month => Resolution::Hour,
            Self::Year => Resolution::Day,
        }
    }

    /// The window `[start, now)`; a bucket counts when it starts inside it.
    #[must_use]
    pub(super) fn start(self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - self.length()
    }
}

/// The beats an aggregate row or a sum of rows holds, by status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Tally {
    pub(super) up: i64,
    pub(super) down: i64,
    pub(super) degraded: i64,
}

/// (up + degraded) / (up + degraded + down); Maintenance and Unknown beats are not counted.
/// `None` when nothing was counted (CHK-011).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "beat counts stay far below 2^52, where f64 is exact"
)]
pub(super) fn ratio(tally: Tally) -> Option<f64> {
    let good = tally.up + tally.degraded;
    let counted = good + tally.down;
    (counted > 0).then(|| good as f64 / counted as f64)
}
