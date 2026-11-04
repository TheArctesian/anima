//! The login throttle rule (D-8): five failed attempts for one username or one client address
//! within 15 minutes refuse further attempts for the 15 minutes after the fifth.
//!
//! Pure: the store passes the failure times, the caller passes `now`.

use chrono::{DateTime, TimeDelta, Utc};

/// Failures that start a lockout.
const FAILURES: usize = 5;

/// The failures must fall within this span.
const WINDOW: TimeDelta = TimeDelta::minutes(15);

/// How long a lockout lasts, from the failure that started it.
const LOCKOUT: TimeDelta = TimeDelta::minutes(15);

/// Failures older than this cannot affect `now`; the store reads only the newer ones.
pub(super) const LOOKBACK: TimeDelta = WINDOW.checked_add(&LOCKOUT).expect("30 minutes");

/// Whether the failures of one username or of one address lock it at `now`.
///
/// `failures` are the times of checked failed attempts (refused attempts excluded, so
/// retrying during a lockout does not extend it), oldest first.
fn locked(failures: &[DateTime<Utc>], now: DateTime<Utc>) -> bool {
    failures.windows(FAILURES).any(|run| {
        let (first, fifth) = (run[0], run[FAILURES - 1]);
        fifth - first <= WINDOW && now < fifth + LOCKOUT
    })
}

/// Whether an attempt at `now` is refused, from the failure times of its username and of its
/// client address, each oldest first.
pub(super) fn refused(
    username_failures: &[DateTime<Utc>],
    client_failures: &[DateTime<Utc>],
    now: DateTime<Utc>,
) -> bool {
    locked(username_failures, now) || locked(client_failures, now)
}
