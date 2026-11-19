//! The only SQL for the operational key tables of `credentials` (CQ-032; K-E5 Section 2): the
//! keys and their versions here, the consumers in [`consumers`], the reads that build a key in
//! [`reads`]. The sealed column is written once and read only by [`fetchable`]; no other
//! statement selects it (LOG-007).

pub(super) mod consumers;
pub(super) mod reads;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::model::{Metadata, VersionState};
use crate::ids::{DeviceId, OperationalKeyId, OperationalKeyVersionId};

const NAME_KEY: &str = "operational_keys_name_key";

/// Who made a row, and when.
pub(super) type Made<'a> = (DateTime<Utc>, &'a str);

/// Inserts a key; `false` when a key, removed or not, already has the name.
pub(super) async fn insert_key(
    conn: &mut PgConnection,
    (id, name): (OperationalKeyId, &str),
    metadata: &Metadata,
    (at, by): Made<'_>,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query(
        "INSERT INTO credentials.operational_keys (id, name, provider, purpose, instructions, \
         provider_link, rotation_days, expires_at, created_at, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(name)
    .bind(&metadata.provider)
    .bind(&metadata.purpose)
    .bind(&metadata.instructions)
    .bind(&metadata.provider_link)
    .bind(metadata.rotation_days)
    .bind(metadata.expires_at)
    .bind(at)
    .bind(by)
    .execute(conn)
    .await;
    match done {
        Ok(_) => Ok(true),
        Err(error) if is_constraint(&error, NAME_KEY) => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_constraint(error: &sqlx::Error, name: &str) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.constraint())
        == Some(name)
}

/// The id of the key named `name` that is not removed, locked until the transaction ends, so
/// two changes of one key run one after the other.
pub(super) async fn lock_key(
    conn: &mut PgConnection,
    name: &str,
) -> Result<Option<OperationalKeyId>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM credentials.operational_keys \
         WHERE name = $1 AND removed_at IS NULL FOR UPDATE",
    )
    .bind(name)
    .fetch_optional(conn)
    .await
}

/// Whether the key passed its expiry at `now`.
pub(super) async fn is_expired(
    conn: &mut PgConnection,
    id: OperationalKeyId,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT coalesce(expires_at <= $2, false) FROM credentials.operational_keys WHERE id = $1",
    )
    .bind(id)
    .bind(now)
    .fetch_one(conn)
    .await
}

/// Replaces the metadata of the key.
pub(super) async fn update_metadata(
    conn: &mut PgConnection,
    id: OperationalKeyId,
    metadata: &Metadata,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE credentials.operational_keys SET provider = $2, purpose = $3, \
         instructions = $4, provider_link = $5, rotation_days = $6, expires_at = $7 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&metadata.provider)
    .bind(&metadata.purpose)
    .bind(&metadata.instructions)
    .bind(&metadata.provider_link)
    .bind(metadata.rotation_days)
    .bind(metadata.expires_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Marks the key removed at `at`. Its name stays taken.
pub(super) async fn mark_removed(
    conn: &mut PgConnection,
    id: OperationalKeyId,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE credentials.operational_keys SET removed_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(conn)
        .await?;
    Ok(())
}

/// Ends, as `expired` at the expiry of their key, the active versions of the keys that passed
/// their expiry at `now`: of the key `only`, or of every key. A version that ended is never
/// active again, even when the operator moves the expiry later.
pub(super) async fn settle_expired(
    conn: &mut PgConnection,
    only: Option<OperationalKeyId>,
    now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let done = sqlx::query(
        "UPDATE credentials.operational_key_versions v SET state = 'expired', \
         ended_at = k.expires_at FROM credentials.operational_keys k \
         WHERE v.key_id = k.id AND v.state = 'active' AND k.expires_at <= $2 \
         AND ($1::uuid IS NULL OR k.id = $1)",
    )
    .bind(only)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Ends the active version of the key as `state` at `at`; its number, or `None` when no version
/// was active.
pub(super) async fn end_active(
    conn: &mut PgConnection,
    id: OperationalKeyId,
    (state, at): (VersionState, DateTime<Utc>),
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE credentials.operational_key_versions SET state = $2, ended_at = $3 \
         WHERE key_id = $1 AND state = 'active' RETURNING number",
    )
    .bind(id)
    .bind(state.as_sql())
    .bind(at)
    .fetch_optional(conn)
    .await
}

/// A new active version: its id, its key, the bytes of its sealed value, and whether the
/// operator said it was changed at the provider too.
pub(super) struct NewVersion<'a> {
    pub(super) id: OperationalKeyVersionId,
    pub(super) key: OperationalKeyId,
    pub(super) sealed: &'a [u8],
    pub(super) rotated_at_provider: bool,
}

/// Inserts `version` as the active version numbered one past the key's highest; its number.
/// The caller ended the active version first, in the same transaction.
pub(super) async fn insert_version(
    conn: &mut PgConnection,
    version: NewVersion<'_>,
    (at, by): Made<'_>,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO credentials.operational_key_versions (id, key_id, number, sealed, state, \
         created_at, created_by, rotated_at_provider) \
         SELECT $1, $2, coalesce(max(number), 0) + 1, $3, 'active', $4, $5, $6 \
         FROM credentials.operational_key_versions WHERE key_id = $2 RETURNING number",
    )
    .bind(version.id)
    .bind(version.key)
    .bind(version.sealed)
    .bind(at)
    .bind(by)
    .bind(version.rotated_at_provider)
    .fetch_one(conn)
    .await
}

/// The version a device may fetch now: the active version of the key named `name`, while the
/// key is not removed and not past its expiry, and only when `device` is a consumer of it. One
/// statement for the three refusals, so each takes the same path (K-E5 Section 2.1). The key
/// row is held shared until the transaction ends: a rotation or a revoke, which hold it for
/// update, wait, so a fetch never gives or records a version that has ended (R-147).
pub(super) struct Fetchable {
    pub(super) key: OperationalKeyId,
    pub(super) version: OperationalKeyVersionId,
    pub(super) number: i32,
    pub(super) sealed: Vec<u8>,
}

pub(super) async fn fetchable(
    conn: &mut PgConnection,
    (device, name): (DeviceId, &str),
    now: DateTime<Utc>,
) -> Result<Option<Fetchable>, sqlx::Error> {
    let row: Option<(OperationalKeyId, OperationalKeyVersionId, i32, Vec<u8>)> = sqlx::query_as(
        "SELECT k.id, v.id, v.number, v.sealed FROM credentials.operational_keys k \
         JOIN credentials.operational_key_versions v ON v.key_id = k.id AND v.state = 'active' \
         WHERE k.name = $2 AND k.removed_at IS NULL \
         AND (k.expires_at IS NULL OR k.expires_at > $3) \
         AND EXISTS (SELECT 1 FROM credentials.operational_key_consumers c \
                     WHERE c.key_id = k.id AND c.device_id = $1) \
         FOR SHARE OF k",
    )
    .bind(device)
    .bind(name)
    .bind(now)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|(key, version, number, sealed)| Fetchable {
        key,
        version,
        number,
        sealed,
    }))
}

/// Replaces the sealed value of each version that ended before `before` with empty bytes;
/// how many it emptied (K-E5 Section 2: deleted 30 days after the version ended).
pub(super) async fn forget_ended(
    conn: &mut PgConnection,
    before: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let done = sqlx::query(
        "UPDATE credentials.operational_key_versions SET sealed = ''::bytea \
         WHERE state <> 'active' AND ended_at < $1 AND length(sealed) > 0",
    )
    .bind(before)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// A fresh id for a key or a version.
pub(super) fn new_id<T: From<Uuid>>() -> T {
    Uuid::now_v7().into()
}
