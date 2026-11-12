//! The warning marks: which warning events the hub has written (VPN-010, VPN-011, BD-4). The
//! only SQL on the `vpn` tables.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::warnings::Warning;

/// Inserts the mark; true when it is new, so the caller writes the event in the same transaction.
pub(super) async fn mark(
    conn: &mut PgConnection,
    warning: &Warning,
    at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let inserted: Option<i32> = sqlx::query_scalar(
        "insert into vpn.warnings (node_key, kind, key_expiry, written_at) \
         values ($1, $2, $3, $4) \
         on conflict on constraint warnings_once do nothing \
         returning 1",
    )
    .bind(&warning.key)
    .bind(warning.kind.as_str())
    .bind(warning.key_expiry)
    .bind(at)
    .fetch_optional(conn)
    .await?;
    Ok(inserted.is_some())
}
