//! The hub's daily jobs, started once by `main.rs`: each only calls its domains (D-4, DB-004,
//! DB-005, D-25).

use std::sync::Arc;

use anima_hub::{
    access, checks,
    clock::Clock,
    events, metrics, notifications,
    retention::{self, Subject},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Starts the daily jobs, each run now and then daily: `retention` makes the day partitions
/// ahead and drops the expired ones, and `access_prune` deletes old login attempts and expired
/// tokens (D-4, DB-004, DB-005, D-25).
pub fn spawn(pool: &PgPool, clock: &Arc<dyn Clock>) -> [tokio::task::JoinHandle<()>; 2] {
    let retention = {
        let db = pool.clone();
        events::run_daily("retention", clock.clone(), move |now| {
            let db = db.clone();
            async move { keep(&db, now).await }
        })
    };
    let prune = {
        let db = pool.clone();
        events::run_daily("access_prune", clock.clone(), move |now| {
            let db = db.clone();
            async move {
                let pruned = access::prune(&db, now).await;
                pruned.map(drop).map_err(|e| e.fault().unwrap_or("access"))
            }
        })
    };
    [tokio::spawn(retention), tokio::spawn(prune)]
}

/// Reads the periods once, then runs each domain's retention job with its own. A failed read
/// skips the run, so a period is never guessed. A failed job does not stop the others; the
/// first fault is returned and the whole run is retried.
async fn keep(db: &PgPool, now: DateTime<Utc>) -> Result<(), &'static str> {
    let periods = retention::periods(db).await.map_err(|e| e.fault())?;
    let checks = checks::drop_expired(db, now, periods.keep(Subject::Heartbeats))
        .await
        .map_err(|e| e.fault());
    let event_logs = events::keep_partitions(db, now, periods.keep(Subject::Events))
        .await
        .map_err(|e| e.fault());
    let samples = metrics::keep_partitions(db, now, periods.keep(Subject::Metrics))
        .await
        .map_err(|e| e.fault());
    let deliveries = notifications::keep_deliveries(db, now, periods.keep(Subject::Deliveries))
        .await
        .map_err(|e| e.fault());
    checks.and(event_logs).and(samples).and(deliveries)
}

#[cfg(test)]
mod tests;
