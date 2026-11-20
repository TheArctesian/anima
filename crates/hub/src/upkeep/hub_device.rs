//! A hosted hub is a device of its own list (HMD-045, R-137): at the start it makes sure that
//! its device and the check `Hub running` exist, reports the check as Up, and then reports it
//! again at each half of the check's interval, so the device is Up while the hub runs. It records
//! the Linux distribution of its container on its device (HMD-046). When the
//! daemon of its container is not enrolled yet, it writes one enrolment code of its device to
//! the private file that `anima-local-hub` named (HMD-071, HM-T35).

use std::{path::Path, sync::Arc, time::Duration};

use anima_hub::{
    clock::Clock,
    config::Config,
    fleet::{checks, devices, events::Bus, metrics},
    ids::{CheckId, DeviceId},
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
    let (device, check) = match ensure(db, bus, clock.as_ref()).await {
        Ok(made) => made,
        Err(fault) => {
            job_failed(JOB, fault);
            return None;
        }
    };
    if let Some(distribution) = own_distribution(Path::new("/")).await
        && devices::record_distribution(db, device, &distribution)
            .await
            .is_err()
    {
        job_failed(JOB, "distribution");
    }
    let code_file = config
        .own_daemon
        .as_ref()
        .and_then(|own| own.code_file.as_deref());
    if let Some(file) = code_file {
        write_own_code(db, device, clock.as_ref(), file).await;
    }
    let alive = Alive {
        db: db.clone(),
        bus: bus.clone(),
        clock: clock.clone(),
        check,
    };
    alive.report().await;
    Some(alive)
}

/// The files that name the distribution, in the order of os-release(5).
const OS_RELEASE: [&str; 2] = ["etc/os-release", "usr/lib/os-release"];

/// The distribution of the system under `root` (`/` for the container of the hub): the first
/// file of [`OS_RELEASE`] that reads; `None` when it names none.
pub(super) async fn own_distribution(root: &Path) -> Option<devices::Distribution> {
    for file in OS_RELEASE {
        if let Ok(text) = tokio::fs::read(root.join(file)).await {
            return metrics::distribution_of(&text);
        }
    }
    None
}

/// Makes a code for the daemon of the container and writes it to `file`, mode 0600, in place of
/// a file that an earlier start left. A failure is logged by its fault word: the daemon then
/// does not enrol, and the next start tries again. The code reaches no log (LOG-007).
async fn write_own_code(db: &PgPool, device: DeviceId, clock: &dyn Clock, file: &Path) {
    let Ok(code) = metrics::own_daemon_code(db, device, clock.now()).await else {
        return job_failed(JOB, "own_daemon_code");
    };
    let line = zeroize::Zeroizing::new(format!("{}\n", code.expose()));
    let _ = tokio::fs::remove_file(file).await;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let written = match options.open(file).await {
        Ok(mut opened) => tokio::io::AsyncWriteExt::write_all(&mut opened, line.as_bytes()).await,
        Err(error) => Err(error),
    };
    if written.is_err() {
        let _ = tokio::fs::remove_file(file).await;
        job_failed(JOB, "own_daemon_code_file");
    }
}

/// The device and its check, added in one transaction or not at all.
async fn ensure(
    db: &PgPool,
    bus: &Bus,
    clock: &dyn Clock,
) -> Result<(DeviceId, CheckId), &'static str> {
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
    Ok((device.id, check))
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
