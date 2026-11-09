//! The hub's scheduled jobs, started once by `main.rs`: each only calls its domains (D-4,
//! DB-004, DB-005, D-25; WP-36 D-2).

use std::sync::Arc;

use anima_hub::{
    access, builds, checks,
    clock::Clock,
    events::{self, Bus},
    metrics, notifications,
    retention::{self, Subject},
    schema::Dependencies,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

/// Starts the scheduled jobs: the daily ones, each run now and then daily (`retention` makes the
/// day partitions ahead and drops the expired ones, and `access_prune` deletes old login attempts
/// and expired tokens; D-4, DB-004, DB-005, D-25), and every 30 seconds the end of build leases
/// that ran out (D-2).
pub fn spawn(dependencies: &Dependencies) -> [tokio::task::JoinHandle<()>; 3] {
    let (pool, clock) = (&dependencies.pool, &dependencies.clock);
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
    [
        tokio::spawn(retention),
        tokio::spawn(prune),
        tokio::spawn(build_leases(
            pool.clone(),
            dependencies.bus.clone(),
            dependencies.build_updates.clone(),
            clock.clone(),
        )),
    ]
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

/// Every 30 seconds, ends the build leases that ran out, so the Builds view is true without a
/// claim (D-2). A failure is written to the process log by its fault word only, and the next
/// tick tries again.
async fn build_leases(db: PgPool, bus: Bus, updates: builds::Updates, clock: Arc<dyn Clock>) {
    let mut ticks = tokio::time::interval(builds::LEASE_SWEEP);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        let queue = builds::BuildQueue {
            db: &db,
            bus: &bus,
            updates: &updates,
        };
        if let Err(error) = builds::expire_leases(queue, clock.now()).await {
            let fault = error.fault().unwrap_or("builds");
            tracing::warn!(job = "build_leases", fault, "scheduled job failed");
        }
    }
}

#[cfg(test)]
mod tests;
