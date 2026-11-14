//! All SQL on schema `extensions`: I/O only.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::model::Key;

/// The keys whose switch row says on.
pub(super) async fn switched_on(db: impl PgExecutor<'_>) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT key FROM extensions.switches WHERE is_on")
        .fetch_all(db)
        .await
}

/// Every row: the key and when it last changed.
pub(super) async fn changes(
    db: impl PgExecutor<'_>,
) -> Result<Vec<(String, DateTime<Utc>)>, sqlx::Error> {
    sqlx::query_as("SELECT key, changed_at FROM extensions.switches")
        .fetch_all(db)
        .await
}

/// Blocks other writers until the transaction ends, so two requests cannot both read the
/// same switches and plan from them. Readers are not blocked.
pub(super) async fn lock(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("LOCK TABLE extensions.switches IN EXCLUSIVE MODE")
        .execute(conn)
        .await
        .map(drop)
}

/// Writes the switch of `key`.
pub(super) async fn set(
    conn: &mut PgConnection,
    (key, on): (Key, bool),
    (at, by): (DateTime<Utc>, &str),
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO extensions.switches (key, is_on, changed_at, changed_by) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (key) DO UPDATE \
         SET is_on = $2, changed_at = $3, changed_by = $4",
    )
    .bind(key.as_str())
    .bind(on)
    .bind(at)
    .bind(by)
    .execute(conn)
    .await
    .map(drop)
}
