//! The last bootstrap file read, kept for [`FRESH_FOR`] (D-29).
//!
//! Time comes in as a parameter. The mutex guards only the swap of the kept file and is never
//! held across an await. Attempts that find the file stale at the same moment each read it
//! once; the probe permits of the scheduler bound how many, and the next read is a day later.

use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};

use super::bootstrap::Bootstrap;

/// How long a bootstrap file read is used before the next probe reads it again (D-29).
pub(super) const FRESH_FOR: TimeDelta = TimeDelta::hours(24);

/// The bootstrap file and the instant it was read.
#[derive(Debug, Default)]
pub(super) struct Cache {
    kept: Mutex<Option<(DateTime<Utc>, Arc<Bootstrap>)>>,
}

impl Cache {
    /// The file kept, when it was read less than [`FRESH_FOR`] before or after `now`. A clock
    /// that moved back by a day or more spends it too.
    pub(super) fn fresh(&self, now: DateTime<Utc>) -> Option<Arc<Bootstrap>> {
        let kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.as_ref()
            .filter(|(at, _)| (now - *at).abs() < FRESH_FOR)
            .map(|(_, bootstrap)| Arc::clone(bootstrap))
    }

    /// Keeps `bootstrap`, read at `now`, in place of the file kept before.
    pub(super) fn keep(&self, now: DateTime<Utc>, bootstrap: Arc<Bootstrap>) {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        *kept = Some((now, bootstrap));
    }
}
