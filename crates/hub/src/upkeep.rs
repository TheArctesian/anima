//! The hub's scheduled jobs, started once by `main.rs`: each only calls its domain (D-4,
//! DB-004, DB-005, K2 retention; WP-36 D-2).

use std::sync::Arc;

use anima_hub::{
    access, builds,
    clock::Clock,
    events::{self, Bus},
    metrics,
    schema::Dependencies,
};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

/// Starts the scheduled jobs: the daily ones, each run now and then daily (D-4, DB-004,
/// DB-005, K2 retention): day partitions ahead and expired ones dropped, and old login attempts
/// and expired tokens deleted; and every 30 seconds the end of build leases that ran out (D-2).
pub fn spawn(dependencies: &Dependencies) -> [tokio::task::JoinHandle<()>; 4] {
    let (pool, clock) = (&dependencies.pool, &dependencies.clock);
    let partitions = {
        let db = pool.clone();
        events::run_daily("partitions", clock.clone(), move |now| {
            let db = db.clone();
            async move {
                events::keep_partitions(&db, now)
                    .await
                    .map_err(|e| e.fault())
            }
        })
    };
    let metrics_partitions = {
        let db = pool.clone();
        events::run_daily("metrics_partitions", clock.clone(), move |now| {
            let db = db.clone();
            async move {
                metrics::keep_partitions(&db, now)
                    .await
                    .map_err(|e| e.fault())
            }
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
        tokio::spawn(partitions),
        tokio::spawn(metrics_partitions),
        tokio::spawn(prune),
        tokio::spawn(build_leases(
            pool.clone(),
            dependencies.bus.clone(),
            dependencies.build_updates.clone(),
            clock.clone(),
        )),
    ]
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
