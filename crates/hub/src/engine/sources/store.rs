//! All SQL on schema `sources`: I/O only (K-E1 Section 5). Run rows are in `store/runs.rs`.

pub(super) mod runs;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgExecutor, types::Json};

use super::{
    model::{Account, ErrorKind},
    schedule::Due,
};
use crate::ids::{SourceAccountId, SourceSecretId};

const ACCOUNT: &str = "id, source, label, instance_key, credential_id, last_started_at, \
    last_sync_at, last_error_kind, last_error, failure_count, next_sync_at, paused_at, \
    run_generation, run_until, created_at";

/// A new account, or the replacement of an existing one's key.
pub(super) struct Connecting<'a> {
    pub id: SourceAccountId,
    pub source: &'a str,
    pub label: &'a str,
    pub instance_key: &'a str,
    pub now: DateTime<Utc>,
}

/// A claimed account with its sync state.
#[derive(sqlx::FromRow)]
pub(super) struct Claimed {
    #[sqlx(flatten)]
    pub account: Account,
    pub sync_state: Json<Value>,
}

/// How a run ends, as written to the account.
pub(super) struct Ending {
    pub generation: i64,
    pub now: DateTime<Utc>,
    /// `None` for success; the kind and fixed words for a failure.
    pub failure: Option<(ErrorKind, &'static str)>,
    /// False when the run stopped: the failure count and the last sync stay.
    pub counted: bool,
    pub next_sync_at: DateTime<Utc>,
}

/// Inserts the account of `connecting`, or, when its source already has an account with that
/// instance key, locks it, takes the new label and clears its errors. Returns its id and
/// whether it is new.
pub(super) async fn connect(
    conn: &mut PgConnection,
    connecting: &Connecting<'_>,
) -> Result<(SourceAccountId, bool), sqlx::Error> {
    let inserted: Option<SourceAccountId> = sqlx::query_scalar(
        "INSERT INTO sources.accounts (id, source, label, instance_key, next_sync_at, created_at) \
         VALUES ($1, $2, $3, $4, $5, $5) ON CONFLICT (source, instance_key) DO NOTHING RETURNING id",
    )
    .bind(connecting.id)
    .bind(connecting.source)
    .bind(connecting.label)
    .bind(connecting.instance_key)
    .bind(connecting.now)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = inserted {
        return Ok((id, true));
    }
    let id = sqlx::query_scalar(
        "UPDATE sources.accounts SET label = $3, last_error_kind = NULL, last_error = NULL, \
         failure_count = 0, next_sync_at = $4 \
         WHERE source = $1 AND instance_key = $2 RETURNING id",
    )
    .bind(connecting.source)
    .bind(connecting.instance_key)
    .bind(connecting.label)
    .bind(connecting.now)
    .fetch_one(conn)
    .await?;
    Ok((id, false))
}

/// Points the account at its sealed credential.
pub(super) async fn set_credential(
    conn: &mut PgConnection,
    id: SourceAccountId,
    credential: SourceSecretId,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sources.accounts SET credential_id = $2 WHERE id = $1")
        .bind(id)
        .bind(credential)
        .execute(conn)
        .await
        .map(drop)
}

pub(super) async fn account(
    db: impl PgExecutor<'_>,
    id: SourceAccountId,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ACCOUNT} FROM sources.accounts WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await
}

/// The account, locked until the transaction ends.
pub(super) async fn lock(
    conn: &mut PgConnection,
    id: SourceAccountId,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ACCOUNT} FROM sources.accounts WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await
}

/// Every account, or those of `source`, oldest first.
pub(super) async fn accounts(
    db: impl PgExecutor<'_>,
    source: Option<&str>,
) -> Result<Vec<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ACCOUNT} FROM sources.accounts WHERE $1::text IS NULL OR source = $1 \
         ORDER BY created_at, id"
    ))
    .bind(source)
    .fetch_all(db)
    .await
}

/// Claims the account for a run at `now` until `until`, when it is not paused and no live
/// claim holds it: the generation goes up by one, so an older claim can no longer write.
pub(super) async fn claim(
    conn: &mut PgConnection,
    id: SourceAccountId,
    (now, until): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Option<Claimed>, sqlx::Error> {
    sqlx::query_as(&format!(
        "UPDATE sources.accounts \
         SET run_generation = run_generation + 1, run_until = $3, last_started_at = $2 \
         WHERE id = $1 AND paused_at IS NULL AND (run_until IS NULL OR run_until < $2) \
         RETURNING {ACCOUNT}, sync_state"
    ))
    .bind(id)
    .bind(now)
    .bind(until)
    .fetch_optional(conn)
    .await
}

/// Locks the account while `generation` is still its claim; `None` when another run has
/// claimed it since or it is gone. Returns whether it is paused.
pub(super) async fn lock_claim(
    conn: &mut PgConnection,
    id: SourceAccountId,
    generation: i64,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT paused_at IS NOT NULL FROM sources.accounts \
         WHERE id = $1 AND run_generation = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(generation)
    .fetch_optional(conn)
    .await
}

/// Writes one intake's slice of the sync state, and extends the claim to `until`.
pub(super) async fn write_slice(
    conn: &mut PgConnection,
    id: SourceAccountId,
    (key, slice): (&str, &Value),
    until: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sources.accounts \
         SET sync_state = jsonb_set(sync_state, ARRAY[$2::text], $3, true), run_until = $4 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(key)
    .bind(Json(slice))
    .bind(until)
    .execute(conn)
    .await
    .map(drop)
}

/// Ends the claim of `ending.generation`; `None` when another run holds the account now.
pub(super) async fn end(
    conn: &mut PgConnection,
    id: SourceAccountId,
    ending: &Ending,
) -> Result<Option<Account>, sqlx::Error> {
    let (kind, words) = ending
        .failure
        .map_or((None, None), |(k, w)| (Some(k.as_str()), Some(w)));
    sqlx::query_as(&format!(
        "UPDATE sources.accounts SET run_until = NULL, next_sync_at = $4, \
         last_sync_at = CASE WHEN $5 AND $6::text IS NULL THEN $3 ELSE last_sync_at END, \
         failure_count = CASE WHEN NOT $5 THEN failure_count \
             WHEN $6::text IS NULL THEN 0 ELSE failure_count + 1 END, \
         last_error_kind = CASE WHEN $5 THEN $6 ELSE last_error_kind END, \
         last_error = CASE WHEN $5 THEN $7 ELSE last_error END \
         WHERE id = $1 AND run_generation = $2 RETURNING {ACCOUNT}"
    ))
    .bind(id)
    .bind(ending.generation)
    .bind(ending.now)
    .bind(ending.next_sync_at)
    .bind(ending.counted)
    .bind(kind)
    .bind(words)
    .fetch_optional(conn)
    .await
}

/// The accounts due at `now`: not paused, `next_sync_at` reached, no live claim. Oldest due
/// first.
pub(super) async fn due(
    db: impl PgExecutor<'_>,
    now: DateTime<Utc>,
) -> Result<Vec<Due>, sqlx::Error> {
    let rows: Vec<(SourceAccountId, String)> = sqlx::query_as(
        "SELECT id, source FROM sources.accounts \
         WHERE paused_at IS NULL AND next_sync_at <= $1 AND (run_until IS NULL OR run_until < $1) \
         ORDER BY next_sync_at, id",
    )
    .bind(now)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, source)| Due { id, source })
        .collect())
}

/// Sets or clears the pause; clearing it makes the account due at `now`.
pub(super) async fn set_paused(
    conn: &mut PgConnection,
    id: SourceAccountId,
    (paused, now): (bool, DateTime<Utc>),
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "UPDATE sources.accounts SET \
         paused_at = CASE WHEN $2 THEN coalesce(paused_at, $3) ELSE NULL END, \
         next_sync_at = CASE WHEN $2 OR paused_at IS NULL THEN next_sync_at ELSE $3 END \
         WHERE id = $1 RETURNING {ACCOUNT}"
    ))
    .bind(id)
    .bind(paused)
    .bind(now)
    .fetch_optional(conn)
    .await
}

/// Makes the account due now, in the caller's transaction.
pub(super) async fn request_sync(
    conn: &mut PgConnection,
    id: SourceAccountId,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE sources.accounts SET next_sync_at = least(next_sync_at, now()) WHERE id = $1",
    )
    .bind(id)
    .execute(conn)
    .await
    .map(|done| done.rows_affected() > 0)
}

/// Deletes the account; its runs go with it.
pub(super) async fn delete(
    conn: &mut PgConnection,
    id: SourceAccountId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sources.accounts WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map(drop)
}
