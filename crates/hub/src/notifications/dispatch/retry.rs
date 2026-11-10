//! When a failed notification is tried again (D-16). Pure: the time comes in as a parameter.
//!
//! A queued notification is tried at once. After a failure that is not permanent it waits 1,
//! then 5, then 15, then 60 minutes before the next try, so it is tried five times in all.
//! After the fifth failure, or after the first permanent one, it is given up. A claim counts
//! the try before the send, so a try whose outcome was never written (the hub stopped, or the
//! write failed) still counts.

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

/// The most tries of one notification: the first and one after each wait.
const TRIES: usize = WAITS.len() + 1;

/// What follows `failure` at `now` for a notification tried `tried` times, this try included
/// (each claim counts one).
pub(super) fn after_failure(failure: Failure, tried: u32, now: DateTime<Utc>) -> Next {
    if failure.is_permanent() {
        return Next::GiveUp;
    }
    tried
        .checked_sub(1)
        .and_then(|failed_before| usize::try_from(failed_before).ok())
        .and_then(|failed_before| WAITS.get(failed_before))
        .map_or(Next::GiveUp, |wait| Next::RetryAt(now + *wait))
}

/// Whether a notification claimed for its `tried`th try has no try left: every earlier claim
/// sent, or may have sent, and none wrote an outcome. It is given up unsent, so a send whose
/// outcome cannot be written is not repeated without end (D-16).
pub(super) fn out_of_tries(tried: u32) -> bool {
    usize::try_from(tried).map_or(true, |tried| tried > TRIES)
}
