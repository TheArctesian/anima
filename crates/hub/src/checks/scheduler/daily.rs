//! The daily jobs: heartbeat partitions ahead, old partitions dropped, aggregates trimmed and
//! orphan heartbeats swept (DB-004, DB-005, DB-009, R-004, HUB-005).

use chrono::{DateTime, Days, Utc};
use sqlx::PgPool;

use super::tick::logged;
use crate::checks;

/// Days of heartbeat partitions created after today (DB-004).
const DAYS_AHEAD: u32 = 7;
/// Raw heartbeats are kept 30 days (DB-009).
const HEARTBEAT_DAYS: Days = Days::new(30);

/// Runs every job once; returns whether all succeeded. A failed job is logged and the
/// others still run.
pub(super) async fn run(db: &PgPool, now: DateTime<Utc>) -> bool {
    let ahead = checks::create_days_ahead(db, now, DAYS_AHEAD).await;
    let ahead = logged(None, "the heartbeat partitions cannot be created", ahead).is_some();
    let cutoff = now.date_naive() - HEARTBEAT_DAYS;
    let dropped = checks::drop_heartbeat_days_before(db, cutoff, now).await;
    let dropped = logged(
        None,
        "the old heartbeat partitions cannot be dropped",
        dropped,
    );
    if let Some(names) = dropped.as_ref().filter(|names| !names.is_empty()) {
        tracing::info!(domain = "checks", dropped = ?names, "old heartbeat partitions dropped");
    }
    let trimmed = checks::trim_aggregates(db, now).await;
    let trimmed = logged(None, "the aggregates cannot be trimmed", trimmed).is_some();
    let swept = checks::delete_orphan_heartbeats(db).await;
    let swept = logged(None, "the orphan heartbeats cannot be swept", swept).is_some();
    ahead && dropped.is_some() && trimmed && swept
}
