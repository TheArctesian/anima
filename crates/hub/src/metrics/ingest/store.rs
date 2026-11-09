//! All SQL on `metrics.agents` and `metrics.enrolments` (K2 0710, 0712). Lookups are by hash
//! only; the time is a parameter, and a value is usable until, not at, its expiry.

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

/// Makes `token` the device's only token, with no renewed one beside it. The stale timer
/// starts again: the new agent has sent no batch yet (MET-018).
pub(super) async fn replace_token(
    conn: &mut PgConnection,
    device: DeviceId,
    token: TokenHash,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO metrics.agents (device_id, token_sha256, expires_at) VALUES ($1, $2, $3) \
         ON CONFLICT (device_id) DO UPDATE SET token_sha256 = excluded.token_sha256, \
             expires_at = excluded.expires_at, next_token_sha256 = NULL, next_expires_at = NULL, \
             last_batch_at = NULL, stale_at = NULL",
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

/// Whether the code, used or not, is still stored for the device: a revoke deletes it.
pub(super) async fn code_kept(
    conn: &mut PgConnection,
    code: CodeHash,
    device: DeviceId,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM metrics.enrolments WHERE code_sha256 = $1 AND device_id = $2)",
    )
    .bind(code.to_bytes().as_slice())
    .bind(device)
    .fetch_one(conn)
    .await
}

/// Deletes the device's token and all its codes, used ones too, so an exchange whose code was
/// redeemed before the revoke finds it gone; whether the device had a token.
pub(super) async fn revoke(conn: &mut PgConnection, device: DeviceId) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query("DELETE FROM metrics.agents WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM metrics.enrolments WHERE device_id = $1")
        .bind(device)
        .execute(conn)
        .await?;
    Ok(deleted.rows_affected() > 0)
}

/// Records a batch accepted at `now` from the device whose usable token is still `token`, in
/// the caller's transaction, and clears its stale mark; whether it was marked stale. `None`
/// when the token was revoked or replaced since it was checked (MET-013, MET-018).
pub(super) async fn take_batch(
    conn: &mut PgConnection,
    device: DeviceId,
    token: TokenHash,
    now: DateTime<Utc>,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "WITH old AS ( \
            SELECT device_id, stale_at FROM metrics.agents \
            WHERE device_id = $1 AND token_sha256 = $2 AND expires_at > $3 FOR UPDATE) \
         UPDATE metrics.agents a SET last_batch_at = $3, stale_at = NULL FROM old \
         WHERE a.device_id = old.device_id RETURNING old.stale_at IS NOT NULL",
    )
    .bind(device)
    .bind(token.to_bytes().as_slice())
    .bind(now)
    .fetch_optional(conn)
    .await
}

/// The devices whose agent has sent a batch, none since before `cutoff`, and is not marked
/// stale yet (MET-018).
pub(super) async fn overdue(
    db: &PgPool,
    cutoff: DateTime<Utc>,
) -> Result<Vec<DeviceId>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT device_id FROM metrics.agents \
         WHERE stale_at IS NULL AND last_batch_at < $1 ORDER BY device_id",
    )
    .bind(cutoff)
    .fetch_all(db)
    .await
}

/// Marks the device's agent stale at `now` in the caller's transaction, unless a batch came at
/// or after `cutoff` or it is marked already; whether this marked it.
pub(super) async fn mark_stale(
    conn: &mut PgConnection,
    device: DeviceId,
    (cutoff, now): (DateTime<Utc>, DateTime<Utc>),
) -> Result<bool, sqlx::Error> {
    let marked = sqlx::query(
        "UPDATE metrics.agents SET stale_at = $3 \
         WHERE device_id = $1 AND stale_at IS NULL AND last_batch_at < $2",
    )
    .bind(device)
    .bind(cutoff)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(marked.rows_affected() > 0)
}

/// Of `devices`, those with no fresh agent: no token, no batch yet, or marked stale (MET-018).
pub(super) async fn without_fresh_agent(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<Vec<DeviceId>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT d FROM unnest($1::uuid[]) AS d WHERE NOT EXISTS ( \
            SELECT 1 FROM metrics.agents a WHERE a.device_id = d \
            AND a.last_batch_at IS NOT NULL AND a.stale_at IS NULL)",
    )
    .bind(devices)
    .fetch_all(db)
    .await
}
