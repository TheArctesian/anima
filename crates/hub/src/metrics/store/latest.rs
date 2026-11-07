//! The newest sample of each card series, from the raw samples (D-8).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::rows::LatestRow;
use crate::{
    ids::DeviceId,
    metrics::{
        Error,
        latest::{Latest, READ_KINDS},
    },
};

/// One lateral index scan per series: `(series_id, at)` is the primary key of every partition,
/// so each scan reads one row (API-005).
const LATEST_SQL: &str = "\
    SELECT s.device_id, s.kind, l.at, l.value \
    FROM metrics.series s \
    CROSS JOIN LATERAL ( \
        SELECT at, value FROM metrics.samples \
        WHERE series_id = s.id AND at >= $3 AND at <= $4 \
        ORDER BY at DESC LIMIT 1) l \
    WHERE s.device_id = ANY($1) AND s.kind = ANY($2)";

/// The newest sample of each card series of `devices` taken in `[now - within, now]`, in one
/// query.
pub(in crate::metrics) async fn latest(
    db: &PgPool,
    devices: &[DeviceId],
    now: DateTime<Utc>,
    within: TimeDelta,
) -> Result<Vec<Latest>, Error> {
    let kinds: Vec<&str> = READ_KINDS.iter().map(|kind| kind.as_str()).collect();
    let rows: Vec<LatestRow> = sqlx::query_as(LATEST_SQL)
        .bind(devices)
        .bind(kinds)
        .bind(now - within)
        .bind(now)
        .fetch_all(db)
        .await?;
    rows.into_iter().map(Latest::try_from).collect()
}
