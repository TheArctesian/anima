//! The hub's daily jobs, started once by `main.rs`: each only calls its domain (D-4, DB-004,
//! DB-005, K2 retention).

use std::sync::Arc;

use anima_hub::{access, clock::Clock, events, metrics};
use sqlx::PgPool;

/// Starts the daily jobs, each run now and then daily (D-4, DB-004, DB-005, K2 retention):
/// day partitions ahead and expired ones dropped, and old login attempts and expired tokens
/// deleted.
pub fn spawn(pool: &PgPool, clock: &Arc<dyn Clock>) -> [tokio::task::JoinHandle<()>; 3] {
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
    ]
}
