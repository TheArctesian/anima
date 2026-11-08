//! All SQL on `metrics.agents` and `metrics.enrolments` (K2 0710). Lookups are by hash only;
//! the time is a parameter, and a value is usable until, not at, its expiry.

use agent_core::wire::{CodeHash, TokenHash};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::ids::DeviceId;

/// Deletes the device's unused codes, so only the next one works (K8 Section 3).
pub(super) async fn void_unused_codes(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM metrics.enrolments WHERE device_id = $1 AND used_at IS NULL")
        .bind(device)
        .execute(conn)
        .await?;
    Ok(())
}

pub(super) async fn insert_code(
    conn: &mut PgConnection,
    code: CodeHash,
    device: DeviceId,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO metrics.enrolments (code_sha256, device_id, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(code.to_bytes().as_slice())
    .bind(device)
    .bind(expires_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Marks the code used at `now` in one statement, committed on its own, and returns its device;
/// `None` for a code that is unknown, used or expired. Two uses at once cannot both pass: the
/// second waits for the first's row lock and then finds it used (MET-012).
pub(super) async fn use_code(
    db: &PgPool,
    code: CodeHash,
    now: DateTime<Utc>,
) -> Result<Option<DeviceId>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE metrics.enrolments SET used_at = $2 \
         WHERE code_sha256 = $1 AND used_at IS NULL AND expires_at > $2 RETURNING device_id",
    )
    .bind(code.to_bytes().as_slice())
    .bind(now)
    .fetch_optional(db)
    .await
}

/// Makes `token` the device's only token, with no renewed one beside it.
pub(super) async fn replace_token(
    conn: &mut PgConnection,
    device: DeviceId,
    token: TokenHash,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO metrics.agents (device_id, token_sha256, expires_at) VALUES ($1, $2, $3) \
         ON CONFLICT (device_id) DO UPDATE SET token_sha256 = excluded.token_sha256, \
             expires_at = excluded.expires_at, next_token_sha256 = NULL, next_expires_at = NULL",
    )
    .bind(device)
    .bind(token.to_bytes().as_slice())
    .bind(expires_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The device whose usable token is `token` at `now`. A renewed token is promoted on its first
/// use, which ends the token it replaced (MET-013). The promotion runs first: a second first use
/// at the same moment waits for the row, finds no renewed token, and then finds it current.
pub(super) async fn holder(
    db: &PgPool,
    token: TokenHash,
    now: DateTime<Utc>,
) -> Result<Option<DeviceId>, sqlx::Error> {
    let hash = token.to_bytes();
    let promoted: Option<DeviceId> = sqlx::query_scalar(
        "UPDATE metrics.agents SET token_sha256 = next_token_sha256, \
             expires_at = next_expires_at, next_token_sha256 = NULL, next_expires_at = NULL \
         WHERE next_token_sha256 = $1 AND next_expires_at > $2 RETURNING device_id",
    )
    .bind(hash.as_slice())
    .bind(now)
    .fetch_optional(db)
    .await?;
    if promoted.is_some() {
        return Ok(promoted);
    }
    sqlx::query_scalar(
        "SELECT device_id FROM metrics.agents WHERE token_sha256 = $1 AND expires_at > $2",
    )
    .bind(hash.as_slice())
    .bind(now)
    .fetch_optional(db)
    .await
}

/// Stores `next` as the renewed token of the device whose current, usable token is `current`,
/// replacing an unused renewed token (K8 Section 5); `None` when `current` is no longer that,
/// because the token was revoked or replaced by a new enrolment meanwhile.
pub(super) async fn set_next(
    db: &PgPool,
    current: TokenHash,
    next: TokenHash,
    (now, expires_at): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Option<DeviceId>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE metrics.agents SET next_token_sha256 = $2, next_expires_at = $4 \
         WHERE token_sha256 = $1 AND expires_at > $3 RETURNING device_id",
    )
    .bind(current.to_bytes().as_slice())
    .bind(next.to_bytes().as_slice())
    .bind(now)
    .bind(expires_at)
    .fetch_optional(db)
    .await
}

/// Deletes the device's token and its unused codes; whether it had a token.
pub(super) async fn revoke(conn: &mut PgConnection, device: DeviceId) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query("DELETE FROM metrics.agents WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    void_unused_codes(conn, device).await?;
    Ok(deleted.rows_affected() > 0)
}
