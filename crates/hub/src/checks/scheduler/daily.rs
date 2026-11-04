//! The daily jobs: heartbeat partitions ahead, old partitions dropped, aggregates trimmed and
//! orphan heartbeats swept (DB-004, DB-005, DB-009, R-004, HUB-005).

use chrono::{DateTime, Days, Utc};
use sqlx::PgPool;

use crate::checks::{self, Error};

/// Days of heartbeat partitions created after today (DB-004).
const DAYS_AHEAD: u32 = 7;
/// Raw heartbeats are kept 30 days (DB-009).
const HEARTBEAT_DAYS: Days = Days::new(30);

/// The job's result, with a failure logged.
fn logged<T>(job: &'static str, result: Result<T, Error>) -> Option<T> {
    result
        .inspect_err(|error| {
            tracing::warn!(domain = "checks", job, %error, "a daily checks job failed");
        })
        .ok()
}

/// Runs every job once. A failed job is logged and the others still run.
pub(super) async fn run(db: &PgPool, now: DateTime<Utc>) {
    logged(
        "create partitions",
        checks::create_days_ahead(db, now, DAYS_AHEAD).await,
    );
    let cutoff = now.date_naive() - HEARTBEAT_DAYS;
    let dropped = checks::drop_heartbeat_days_before(db, cutoff, now).await;
    if let Some(dropped) = logged("drop partitions", dropped).filter(|d| !d.is_empty()) {
        tracing::info!(
            domain = "checks",
            ?dropped,
            "old heartbeat partitions dropped"
        );
    }
    logged("trim aggregates", checks::trim_aggregates(db, now).await);
    logged(
        "sweep orphan heartbeats",
        checks::delete_orphan_heartbeats(db).await,
    );
}
