//! When a failed notification is tried again (D-16). Pure: the time comes in as a parameter.
//!
//! A queued notification is tried at once. After a failure that is not permanent it waits 1,
//! then 5, then 15, then 60 minutes before the next try, so it is tried five times in all.
//! After the fifth failure, or after the first permanent one, it is given up.

use chrono::{DateTime, TimeDelta, Utc};

use super::failure::Failure;

/// The waits after the first, second, third and fourth failed try.
pub(super) const WAITS: [TimeDelta; 4] = [
    TimeDelta::minutes(1),
    TimeDelta::minutes(5),
    TimeDelta::minutes(15),
    TimeDelta::minutes(60),
];

/// What becomes of a queued notification after a failed try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Next {
    /// Try again at this time.
    RetryAt(DateTime<Utc>),
    /// Stop trying.
    GiveUp,
}

/// What follows `failure` at `now` for a notification that had already failed `failed_before`
/// times.
pub(super) fn after_failure(failure: Failure, failed_before: u32, now: DateTime<Utc>) -> Next {
    if failure.is_permanent() {
        return Next::GiveUp;
    }
    usize::try_from(failed_before)
        .ok()
        .and_then(|failed| WAITS.get(failed))
        .map_or(Next::GiveUp, |wait| Next::RetryAt(now + *wait))
}
