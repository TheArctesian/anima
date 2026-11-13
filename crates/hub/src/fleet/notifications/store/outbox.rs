//! The outbox: notifications decided and not yet finished (D-16).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::rows::{OUTBOX_COLUMNS, OutboxRow};
use crate::fleet::notifications::{
    Error,
    model::{NewOutbox, Outbox},
};

/// Queues a notification, due at once, in the caller's transaction.
pub(in crate::fleet::notifications) async fn insert_outbox(
    conn: &mut PgConnection,
    row: &NewOutbox,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO notifications.outbox \
         (id, channel_id, check_id, kind, payload, created_at, next_attempt_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $6)",
    )
    .bind(row.id)
    .bind(row.channel)
    .bind(row.check)
    .bind(row.kind.as_str())
    .bind(&row.payload)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

/// Claims up to `limit` unfinished rows due at `now`, oldest due first. Each claimed row is
/// pushed `lease` into the future, so no other claimer takes it while it is being sent, and
/// it comes back by itself if the claimer dies; that lease end is the claim's token
/// (`Outbox::lease_end`). The claim counts the try, so a row whose outcome is never written
/// still runs out of tries. `FOR UPDATE SKIP LOCKED` makes concurrent claimers take disjoint
/// rows.
pub(in crate::fleet::notifications) async fn claim_due(
    db: impl PgExecutor<'_>,
    now: DateTime<Utc>,
    lease: TimeDelta,
    limit: i64,
) -> Result<Vec<Outbox>, Error> {
    let rows: Vec<OutboxRow> = sqlx::query_as(&format!(
        "UPDATE notifications.outbox \
         SET next_attempt_at = $1 + make_interval(secs => $2), attempts = attempts + 1 \
         WHERE id IN ( \
             SELECT id FROM notifications.outbox \
             WHERE sent_at IS NULL AND given_up_at IS NULL AND next_attempt_at <= $1 \
             ORDER BY next_attempt_at, id LIMIT $3 FOR UPDATE SKIP LOCKED) \
         RETURNING {OUTBOX_COLUMNS}"
    ))
    .bind(now)
    .bind(lease.as_seconds_f64())
    .bind(limit)
    .fetch_all(db)
    .await?;
    let mut claimed = rows
        .into_iter()
        .map(Outbox::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    claimed.sort_by_key(|row| row.created_at);
    Ok(claimed)
}

// A claimed row is finished only under its own claim: each statement below matches the row's
// id and the lease end its claim set (`Outbox::lease_end`). A claim whose lease ended, and was
// followed by another claim, finds a different `next_attempt_at` and changes nothing (D-16).
// Every claim moves `next_attempt_at` past the one before it (a row is claimed only once
// `next_attempt_at <= now`, and the claim sets `now + lease`), so no two claims share a token.

/// Marks the row of `claim` sent at `at`; false when the claim no longer holds the row (its
/// lease ended and another claimer took it) or the row is gone or finished.
pub(in crate::fleet::notifications) async fn mark_sent(
    db: impl PgExecutor<'_>,
    claim: &Outbox,
    at: DateTime<Utc>,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "UPDATE notifications.outbox SET sent_at = $3 \
         WHERE id = $1 AND next_attempt_at = $2 AND sent_at IS NULL AND given_up_at IS NULL",
    )
    .bind(claim.id)
    .bind(claim.lease_end)
    .bind(at)
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Tries the row of `claim` again at `next_attempt_at` after a failed try; false
/// when the claim no longer holds the row or the row is gone or finished.
pub(in crate::fleet::notifications) async fn reschedule(
    db: impl PgExecutor<'_>,
    claim: &Outbox,
    next_attempt_at: DateTime<Utc>,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "UPDATE notifications.outbox SET next_attempt_at = $3 \
         WHERE id = $1 AND next_attempt_at = $2 AND sent_at IS NULL AND given_up_at IS NULL",
    )
    .bind(claim.id)
    .bind(claim.lease_end)
    .bind(next_attempt_at)
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Stops trying the row of `claim` at `at`; false when the claim no longer holds the row or
/// the row is gone or finished.
pub(in crate::fleet::notifications) async fn give_up(
    db: impl PgExecutor<'_>,
    claim: &Outbox,
    at: DateTime<Utc>,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "UPDATE notifications.outbox SET given_up_at = $3 \
         WHERE id = $1 AND next_attempt_at = $2 AND sent_at IS NULL AND given_up_at IS NULL",
    )
    .bind(claim.id)
    .bind(claim.lease_end)
    .bind(at)
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Deletes the rows sent or given up before `cutoff`; returns how many.
pub(in crate::fleet::notifications) async fn delete_finished_before(
    db: impl PgExecutor<'_>,
    cutoff: DateTime<Utc>,
) -> Result<u64, Error> {
    let done = sqlx::query(
        "DELETE FROM notifications.outbox \
         WHERE coalesce(sent_at, given_up_at) < $1",
    )
    .bind(cutoff)
    .execute(db)
    .await?;
    Ok(done.rows_affected())
}
