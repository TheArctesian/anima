//! The lease of work that a machine holds: how long a claim or a report holds it, and what a
//! lease that ended with no report costs. One rule for the build queue and for the jobs of the
//! work route (BLD-005, BLD-006, PRO-050). Pure: the time is a parameter (CQ-021).

use chrono::{DateTime, TimeDelta, Utc};

/// How long a claim or a report holds the work (BLD-005).
pub const LEASE: TimeDelta = TimeDelta::seconds(90);

/// Leases that may end without a report before the work fails (BLD-006).
pub const MAX_LOST: u8 = 3;

/// The lease end of a claim or a report at `now` (BLD-005).
#[must_use]
pub fn end(now: DateTime<Utc>) -> DateTime<Utc> {
    now + LEASE
}

/// Whether a lease that ends at `ends_at` has ended at `now`. Work that is held with no lease
/// end has no lease to hold it.
#[must_use]
pub fn ended(ends_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    ends_at.is_none_or(|end| now >= end)
}

/// What comes after a lease ended with no report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lost {
    /// Back in the queue, with this count of lost leases.
    Again(u8),
    /// The work fails: this was lease [`MAX_LOST`].
    Failed(u8),
}

/// The work after one more lease ended, when `lost` leases ended before it (BLD-006).
#[must_use]
pub fn lost(lost: u8) -> Lost {
    let count = lost.saturating_add(1).min(MAX_LOST);
    if count >= MAX_LOST {
        Lost::Failed(count)
    } else {
        Lost::Again(count)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_800_000_000 + seconds, 0).unwrap()
    }

    #[test]
    fn bld_005_a_lease_holds_for_90_seconds_and_ends_at_its_end() {
        assert_eq!(end(at(0)), at(90));
        assert!(!ended(Some(at(90)), at(89)));
        assert!(ended(Some(at(90)), at(90)));
        assert!(ended(None, at(0)));
    }

    #[test]
    fn bld_006_the_third_lost_lease_fails_the_work() {
        assert_eq!(lost(0), Lost::Again(1));
        assert_eq!(lost(1), Lost::Again(2));
        assert_eq!(lost(2), Lost::Failed(3));
        assert_eq!(lost(7), Lost::Failed(3));
    }
}
