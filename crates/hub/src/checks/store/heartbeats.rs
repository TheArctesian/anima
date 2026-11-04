//! Heartbeat reads: the raw results of one check, newest first (CHK-003).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::rows::status_of;
use crate::{
    checks::{Error, model::Heartbeat},
    ids::CheckId,
};

type Row = (DateTime<Utc>, String, Option<i32>, Option<String>);

fn heartbeat((at, status, response_time_ms, message): Row) -> Result<Heartbeat, Error> {
    Ok(Heartbeat {
        at,
        status: status_of(&status)?,
        response_time_ms: response_time_ms.and_then(|ms| u32::try_from(ms).ok()),
        message,
    })
}

/// Up to `limit` heartbeats of each of the checks strictly before `before`, newest first, in
/// one statement. The primary key makes `at` unique per check, so it is the cursor.
pub(in crate::checks) async fn pages(
    db: &PgPool,
    checks: &[CheckId],
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<(CheckId, Heartbeat)>, Error> {
    type Paged = (CheckId, DateTime<Utc>, String, Option<i32>, Option<String>);
    let rows: Vec<Paged> = sqlx::query_as(
        "SELECT check_id, at, status, response_time_ms, message FROM ( \
             SELECT check_id, at, status, response_time_ms, message, \
                    row_number() OVER (PARTITION BY check_id ORDER BY at DESC) AS place \
             FROM checks.heartbeats \
             WHERE check_id = ANY($1) AND ($2::timestamptz IS NULL OR at < $2)) paged \
         WHERE place <= $3 ORDER BY check_id, at DESC",
    )
    .bind(checks)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(check, at, status, ms, message)| Ok((check, heartbeat((at, status, ms, message))?)))
        .collect()
}

/// The newest heartbeat of each of the checks that has one, in one query.
pub(in crate::checks) async fn latest(
    db: &PgPool,
    checks: &[CheckId],
) -> Result<Vec<(CheckId, Heartbeat)>, Error> {
    type Latest = (CheckId, DateTime<Utc>, String, Option<i32>, Option<String>);
    let rows: Vec<Latest> = sqlx::query_as(
        "SELECT DISTINCT ON (check_id) check_id, at, status, response_time_ms, message \
             FROM checks.heartbeats WHERE check_id = ANY($1) ORDER BY check_id, at DESC",
    )
    .bind(checks)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(check, at, status, ms, message)| Ok((check, heartbeat((at, status, ms, message))?)))
        .collect()
}
