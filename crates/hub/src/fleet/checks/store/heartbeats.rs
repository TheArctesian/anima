//! Heartbeat reads: the raw results of one check, newest first (CHK-003).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::rows::status_of;
use crate::{
    fleet::checks::{Error, model::Heartbeat},
    ids::CheckId,
};

/// One lateral index scan per check: `(check_id, at)` is the primary key of every partition, so
/// each scan stops after `$3` rows instead of reading the check's whole history (API-005).
pub(in crate::fleet::checks) const PAGES_SQL: &str = "\
    SELECT c.id, h.at, h.status, h.response_time_ms, h.message \
    FROM unnest($1::uuid[]) AS c(id) \
    CROSS JOIN LATERAL ( \
        SELECT at, status, response_time_ms, message FROM checks.heartbeats \
        WHERE check_id = c.id AND at < COALESCE($2::timestamptz, 'infinity') \
        ORDER BY at DESC LIMIT $3) h \
    ORDER BY c.id, h.at DESC";

/// The newest heartbeat of each check, by the same lateral scan with `LIMIT 1` (API-005).
pub(in crate::fleet::checks) const LATEST_SQL: &str = "\
    SELECT c.id, h.at, h.status, h.response_time_ms, h.message \
    FROM unnest($1::uuid[]) AS c(id) \
    CROSS JOIN LATERAL ( \
        SELECT at, status, response_time_ms, message FROM checks.heartbeats \
        WHERE check_id = c.id ORDER BY at DESC LIMIT 1) h";

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
pub(in crate::fleet::checks) async fn pages(
    db: &PgPool,
    checks: &[CheckId],
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<(CheckId, Heartbeat)>, Error> {
    type Paged = (CheckId, DateTime<Utc>, String, Option<i32>, Option<String>);
    let rows: Vec<Paged> = sqlx::query_as(PAGES_SQL)
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
pub(in crate::fleet::checks) async fn latest(
    db: &PgPool,
    checks: &[CheckId],
) -> Result<Vec<(CheckId, Heartbeat)>, Error> {
    type Latest = (CheckId, DateTime<Utc>, String, Option<i32>, Option<String>);
    let rows: Vec<Latest> = sqlx::query_as(LATEST_SQL)
        .bind(checks)
        .fetch_all(db)
        .await?;
    rows.into_iter()
        .map(|(check, at, status, ms, message)| Ok((check, heartbeat((at, status, ms, message))?)))
        .collect()
}
