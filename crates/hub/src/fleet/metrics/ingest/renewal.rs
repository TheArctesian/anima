//! Renewal: a daemon swaps its token for a new one before it expires (K8 Section 5, MET-013).

use chrono::{DateTime, Utc};
use daemon_core::wire::{DaemonToken, Renewed};
use sqlx::PgPool;

use super::{Error, TOKEN_LIFETIME, store};

/// Stores `next` beside `current` as the device's renewed token, usable for 365 days from
/// `now`, replacing an unused renewed token, so a lost answer costs nothing. `current` keeps
/// working until `next` is first used. `None` when `current` is no longer the device's usable
/// token, because it was revoked or replaced by a new enrolment after it was checked.
///
/// # Errors
/// `Database` when the write fails.
pub(super) async fn renew(
    db: &PgPool,
    current: &DaemonToken,
    next: DaemonToken,
    now: DateTime<Utc>,
) -> Result<Option<Renewed>, Error> {
    let expires_at = now + TOKEN_LIFETIME;
    let stored = store::set_next(db, current.hash(), next.hash(), (now, expires_at)).await?;
    Ok(stored.map(|_| Renewed {
        token: next,
        expires_at,
    }))
}
