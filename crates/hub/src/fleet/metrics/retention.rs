//! How long metrics are kept (DB-005, DB-009, HUB-005) and what a retention run removed.

use chrono::{DateTime, DurationRound, NaiveDate, TimeDelta, Utc};

/// The periods after which metrics go. The defaults are those of DB-009; a later setting can
/// change them (HUB-005). The database refuses to drop a sample partition less than 1 day
/// old, so a shorter `samples` period fails the run rather than dropping early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retention {
    pub samples: TimeDelta,
    pub minute: TimeDelta,
    pub hour: TimeDelta,
    pub day: TimeDelta,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            samples: TimeDelta::days(30),
            minute: TimeDelta::days(7),
            hour: TimeDelta::days(90),
            day: TimeDelta::days(730),
        }
    }
}

/// The instants before which each kind of row goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cutoffs {
    /// Sample partitions of the UTC days before this date are dropped.
    pub(super) samples: NaiveDate,
    pub(super) minute: DateTime<Utc>,
    pub(super) hour: DateTime<Utc>,
    pub(super) day: DateTime<Utc>,
}

impl Retention {
    pub(super) fn cutoffs(self, now: DateTime<Utc>) -> Cutoffs {
        Cutoffs {
            samples: (now - self.samples).date_naive(),
            minute: now - self.minute,
            hour: now - self.hour,
            day: now - self.day,
        }
    }
}

/// What a retention run removed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trimmed {
    /// The names of the sample partitions dropped.
    pub partitions: Vec<String>,
    pub minute: u64,
    pub hour: u64,
    pub day: u64,
    /// Samples whose series no longer exists (DEV-015).
    pub orphan_samples: u64,
    /// Series with no aggregate row and no recent sample (D-5).
    pub idle_series: u64,
}

/// Midnight UTC of the day that holds `now`. Partitions are created from it, so the database's
/// window (up to 7 days past its own clock) holds even when `now` is a little ahead of it.
pub(super) fn start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.duration_trunc(TimeDelta::days(1)).unwrap_or(now)
}
