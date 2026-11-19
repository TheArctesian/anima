//! How the versions and the due state of a key read at a time (K-E5 Section 2, R-147). Pure:
//! the time is a parameter (CQ-021).

use chrono::{DateTime, TimeDelta, Utc};

use super::model::{Due, Metadata, VersionState};

/// How long the sealed value of a version that ended is kept (K-E5 Section 2).
pub const ENDED_KEPT: TimeDelta = TimeDelta::days(30);

/// How long before `expires_at` a key reads as expiring (K-E5 Section 2).
const EXPIRING_SPAN: TimeDelta = TimeDelta::days(14);

/// Whether a key whose expiry is `expires_at` has expired at `now`.
#[must_use]
pub fn has_expired(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    expires_at.is_some_and(|expiry| expiry <= now)
}

/// The state a version stored as `stored` reads as at `now`: an active version of a key that
/// passed its expiry reads as expired; any other reads as stored.
#[must_use]
pub fn read_state(
    stored: VersionState,
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> VersionState {
    if stored == VersionState::Active && has_expired(expires_at, now) {
        VersionState::Expired
    } else {
        stored
    }
}

/// The due state of a key whose newest version was recorded at `last_rotation`, at `now`:
/// expired, then expiring, then due, in that order.
#[must_use]
pub fn due(metadata: &Metadata, last_rotation: DateTime<Utc>, now: DateTime<Utc>) -> Due {
    if has_expired(metadata.expires_at, now) {
        return Due::Expired;
    }
    if metadata
        .expires_at
        .is_some_and(|expiry| expiry - now <= EXPIRING_SPAN)
    {
        return Due::Expiring;
    }
    let overdue = metadata
        .rotation_days
        .is_some_and(|days| now - last_rotation > TimeDelta::days(i64::from(days)));
    if overdue { Due::Due } else { Due::Current }
}

/// The instant before which an ended version's sealed value is deleted, at `now`.
#[must_use]
pub fn forget_before(now: DateTime<Utc>) -> DateTime<Utc> {
    now - ENDED_KEPT
}
