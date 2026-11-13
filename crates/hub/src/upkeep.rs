//! Starts the hub's background work: periodic jobs and the check, metrics and notification
//! loops (R-072). Each only calls its domains (D-4, DB-004, DB-005, D-25; WP-36 D-2).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::Clock,
    fleet::{
        access, builds, checks,
        credentials::Vault,
        events::{self, Bus},
        metrics, notifications,
        retention::{self, Subject},
        targets::AddressPolicy,
        vpn,
    },
    schema::Dependencies,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

/// Starts the scheduled jobs: the daily ones, each run now and then daily (`retention` makes the
/// day partitions ahead and drops the expired ones, and `access_prune` deletes old login attempts
/// and expired tokens, and audits each builder that went with its token; D-4, DB-004, DB-005,
/// D-25, BLD-025), every 30 seconds the end of build leases that ran out (WP-36 D-2), and, when
/// the hub has a release bucket, every 10 seconds the publisher (WP-36 D-25).
pub fn spawn(dependencies: &Dependencies) -> Vec<tokio::task::JoinHandle<()>> {
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
                let builders = builds::builders(&db).await;
                let before = builders.map_err(|e| e.fault().unwrap_or("builds"))?;
                access::prune(&db, now)
                    .await
                    .map_err(|e| e.fault().unwrap_or("access"))?;
                builds::audit_removed(&db, &before, now, builds::Removal::TokenExpired)
                    .await
                    .map(drop)
                    .map_err(|e| e.fault().unwrap_or("builds"))
            }
        })
    };
    let builds = Builds {
        db: pool.clone(),
        bus: dependencies.bus.clone(),
        updates: dependencies.build_updates.clone(),
        clock: clock.clone(),
    };
    let mut jobs = vec![tokio::spawn(retention), tokio::spawn(prune)];
    jobs.push(tokio::spawn(publishing(
        builds.clone(),
        dependencies.bucket.clone(),
    )));
    jobs.push(tokio::spawn(build_leases(builds)));
    jobs.extend(loops(dependencies));
    jobs
}

/// Starts the loops that watch the devices and send: the check scheduler, the metrics
/// collector and stale watch, the notification dispatcher, and, with a `LocalAPI` proxy, the
/// VPN reader (R-072, VPN-008).
fn loops(dependencies: &Dependencies) -> Vec<tokio::task::JoinHandle<()>> {
    let (pool, bus, clock) = (&dependencies.pool, &dependencies.bus, &dependencies.clock);
    let (probing, collecting, sending) = reaching(
        &dependencies.vault,
        &dependencies.policy,
        dependencies.config.tailscale_localapi.clone(),
    );
    let mut loops = vec![
        checks::spawn_scheduler(pool.clone(), bus.clone(), clock.clone(), probing),
        metrics::spawn_collector(
            pool.clone(),
            dependencies.metrics_feed.clone(),
            bus.clone(),
            clock.clone(),
            collecting,
        ),
        notifications::spawn_dispatcher(pool.clone(), bus.clone(), clock.clone(), sending),
        metrics::spawn_stale_watch(pool.clone(), bus.clone(), clock.clone()),
    ];
    let reading = vpn::Reading {
        pool: pool.clone(),
        bus: bus.clone(),
        clock: clock.clone(),
        feed: dependencies.vpn_feed.clone(),
        broker: dependencies.vpn_broker.clone(),
    };
    loops.extend(vpn::spawn_reader(
        dependencies.config.tailscale_localapi.clone(),
        reading,
    ));
    loops
}

/// What checks, metrics and notifications need to reach the outside: the vault and the one
/// address policy (P2-004, D-16).
fn reaching(
    vault: &Vault,
    policy: &AddressPolicy,
    localapi: Option<String>,
) -> (checks::Probing, metrics::Collecting, notifications::Sending) {
    let (vault, policy) = (vault.clone(), policy.clone());
    (
        checks::Probing {
            vault: vault.clone(),
            policy: policy.clone(),
            localapi,
        },
        metrics::Collecting {
            vault: vault.clone(),
            policy: policy.clone(),
        },
        notifications::Sending { vault, policy },
    )
}

/// What the build jobs of the upkeep share.
#[derive(Clone)]
struct Builds {
    db: PgPool,
    bus: Bus,
    updates: builds::Updates,
    clock: Arc<dyn Clock>,
}

impl Builds {
    const fn queue(&self) -> builds::BuildQueue<'_> {
        builds::BuildQueue {
            db: &self.db,
            bus: &self.bus,
            updates: &self.updates,
        }
    }
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
async fn build_leases(builds: Builds) {
    let mut ticks = tokio::time::interval(builds::LEASE_SWEEP);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        if let Err(error) = builds::expire_leases(builds.queue(), builds.clock.now()).await {
            let fault = error.fault().unwrap_or("builds");
            tracing::warn!(job = "build_leases", fault, "scheduled job failed");
        }
    }
}

/// Every 10 seconds, publishes the build jobs whose files are in (WP-36 D-25), or, with no
/// bucket, fails the jobs handed over before publishing was turned off. The client follows no
/// redirect, so the public check is of the public address itself (BLD-064). A failure is
/// logged by its fault word only; the job is taken again after its hold.
async fn publishing(builds: Builds, bucket: Option<Arc<builds::Bucket>>) {
    let Ok(http) = publishing_client() else {
        job_failed("build_publishing", "http client");
        return;
    };
    let mut ticks = tokio::time::interval(builds::PUBLISH_SWEEP);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        let swept = match &bucket {
            Some(bucket) => {
                builds::publish_ready(builds.queue(), bucket, &http, builds.clock.as_ref()).await
            }
            None => builds::publishing_off(builds.queue(), builds.clock.now()).await,
        };
        if let Err(error) = swept {
            job_failed("build_publishing", error.fault().unwrap_or("builds"));
        }
    }
}

/// Writes a failed run of `job` to the process log by its fault word only (LOG-007).
fn job_failed(job: &'static str, fault: &'static str) {
    tracing::warn!(job, fault, "scheduled job failed");
}

/// The HTTP client of the publisher: no redirect, and a stalled read ends the request.
fn publishing_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(60))
        .build()
}

#[cfg(test)]
mod tests;
