//! The health of an account from its failure count and its last sync (SRC-009, K-E1 Section 9).
//! Pure: the time is a parameter.

use chrono::{DateTime, TimeDelta, Utc};

use super::model::{ErrorKind, Health, PauseReason};

/// The failure count from which an account is stalled.
const STALLED_FAILURES: u32 = 3;
/// How many poll intervals without a success make an account stalled.
const STALLED_INTERVALS: i32 = 3;

/// What the health of an account is judged from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthFacts {
    /// The operator paused it.
    pub paused: bool,
    /// An extension that uses its source is on.
    pub consumer_on: bool,
    pub last_error_kind: Option<ErrorKind>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub failure_count: u32,
    /// The poll interval of the source.
    pub interval: TimeDelta,
}

/// The health at `now`. The first rule that matches wins.
#[must_use]
pub fn health(facts: &HealthFacts, now: DateTime<Utc>) -> Health {
    if facts.paused || !facts.consumer_on {
        return Health::Paused;
    }
    if facts.last_error_kind == Some(ErrorKind::Auth) {
        return Health::NeedsReconnect;
    }
    let failing = facts.failure_count >= STALLED_FAILURES;
    let Some(last_sync) = facts.last_sync_at else {
        return if failing {
            Health::Stalled
        } else if facts.last_started_at.is_some() {
            Health::Starting
        } else {
            Health::NeverSynced
        };
    };
    if failing || now - last_sync > facts.interval * STALLED_INTERVALS {
        Health::Stalled
    } else {
        Health::Ok
    }
}

/// Who paused it, when the health is `Paused`: the operator before an extension that is off.
#[must_use]
pub fn paused_by(facts: &HealthFacts) -> Option<PauseReason> {
    if facts.paused {
        Some(PauseReason::Operator)
    } else if !facts.consumer_on {
        Some(PauseReason::ExtensionOff)
    } else {
        None
    }
}

/// Whether moving from `before` to `after` is a change the operator must hear about: into
/// `Stalled` or `NeedsReconnect` (K-E1 Section 8, step 5).
#[must_use]
pub fn worsened(before: Health, after: Health) -> bool {
    before != after && matches!(after, Health::Stalled | Health::NeedsReconnect)
}
