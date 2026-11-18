//! A hosted hub is a device of its own list (HMD-045, R-137): at the start it makes sure that
//! its device and the check `Hub running` exist, reports the check as Up, and then reports it
//! again at each half of the check's interval, so the device is Up while the hub runs.

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::Clock,
    config::Config,
    fleet::{checks, devices, events::Bus},
    ids::CheckId,
};
use sqlx::PgPool;

use super::job_failed;

const JOB: &str = "hub_device";
/// The wait before the next report when the check is gone or a report failed.
const RETRY: Duration = Duration::from_secs(30);

/// What the reports of the hub's own check need.
pub struct Alive {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    check: CheckId,
}

/// The wait before the next report: half of the interval in which the check waits for one, so
/// one late report does not make the check Down. [`RETRY`] when there is no interval.
pub(super) fn next_report(interval: Option<Duration>) -> Duration {
    interval.map_or(RETRY, |interval| interval / 2)
}

/// In the hosted mode, adds the hub's device and its check when they are not there, and
/// reports the check once, before the scheduler can find it due. `None` for a hub that is not
/// hosted. A failure is logged by its fault word and does not stop the start: the hub serves
/// without its own row, and the next start tries again.
pub async fn start(
    config: &Config,
    db: &PgPool,
    bus: &Bus,
    clock: &Arc<dyn Clock>,
) -> Option<Alive> {
    config.hosted_cloud_key_file.as_ref()?;
    let check = match ensure(db, bus, clock.as_ref()).await {
        Ok(check) => check,
        Err(fault) => {
            job_failed(JOB, fault);
            return None;
        }
    };
    let alive = Alive {
        db: db.clone(),
        bus: bus.clone(),
        clock: clock.clone(),
        check,
    };
    alive.report().await;
    Some(alive)
}

/// The device and its check, added in one transaction or not at all.
async fn ensure(db: &PgPool, bus: &Bus, clock: &dyn Clock) -> Result<CheckId, &'static str> {
    let mut tx = db.begin().await.map_err(|_| "database")?;
    let (device, added) = devices::ensure_hub_device(&mut tx, bus, clock.now())
        .await
        .map_err(|error| match error {
            devices::Error::Database(_) => "database",
            _ => "devices",
        })?;
    let check = checks::own::ensure(&mut tx, device.id, checks::OwnCheck::HubRunning)
        .await
        .map_err(|error| error.fault())?;
    tx.commit().await.map_err(|_| "database")?;
    if added {
        tracing::info!(job = JOB, "the hub added itself to its devices");
    }
    Ok(check)
}

impl Alive {
    /// Reports the check as Up now and returns the wait before the next report.
    async fn report(&self) -> Duration {
        let now = self.clock.now();
        let up = checks::OwnReport::Up;
        let reported = checks::own::report((&self.db, &self.bus), self.check, up, now).await;
        let interval = reported.unwrap_or_else(|error| {
            job_failed(JOB, error.fault());
            None
        });
        next_report(interval)
    }

    /// Reports the check for as long as the hub runs.
    pub async fn keep_reporting(self) {
        loop {
            let wait = self.report().await;
            tokio::time::sleep(wait).await;
        }
    }
}
