//! A hosted hub keeps the check `Connected` for each device whose host is the link address
//! of a computer on its link (HMD-047, R-143). Each 15 seconds it asks the link when it last
//! saw each computer: a computer seen in the last 45 seconds is reported Up. A computer that
//! was seen and is not seen any more is reported Down one time; after that the check gets
//! no report, as a PUSH check whose device is off. A computer that the link never saw gets
//! no check: its link program does not say that it is connected.
//!
//! The time in which the hub slept or did not run is not a Down. At a start and after a
//! sleep, each check waits one whole interval again, and no computer is reported Down until
//! the hub has watched for 45 seconds: the computers need that time to connect again.

use std::{collections::HashSet, net::IpAddr, path::PathBuf, sync::Arc, time::Duration};

use anima_hub::{
    clock::Clock,
    config::Config,
    fleet::{
        checks::{self, OwnCheck, OwnReport},
        devices::{self, Device},
        events::Bus,
        link::{self, Presence},
    },
    ids::{CheckId, DeviceId},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use super::job_failed;

const JOB: &str = "link_presence";
/// The time between two questions to the link. A computer says that it is connected each
/// 15 seconds.
const EVERY: Duration = Duration::from_secs(15);
/// How old the last sign of a computer can be while the computer counts as connected:
/// three of its signs.
const FRESH: TimeDelta = TimeDelta::seconds(45);
/// A wait between two passes longer than this means that the hub slept.
const ASLEEP: TimeDelta = TimeDelta::seconds(45);
/// What the history of the check shows for a computer that left the link.
const GONE: &str = "Not connected";

/// What the link's last sign of a computer means now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Sign {
    /// The computer is connected.
    Here,
    /// The computer was connected and is not any more.
    Gone,
    /// The link never saw the computer, or the hub did not watch long enough to know.
    Unknown,
}

/// What `seen`, the time of the last sign of a computer, means at `now` for a hub that
/// watches since `watching_since`.
pub(super) fn sign(
    seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    watching_since: DateTime<Utc>,
) -> Sign {
    match seen {
        Some(at) if now - at <= FRESH => Sign::Here,
        Some(_) if now - watching_since >= FRESH => Sign::Gone,
        _ => Sign::Unknown,
    }
}

/// The devices of the hub that are computers of the link, each with the time of its last
/// sign: a device whose host is an IP address that the link lists.
pub(super) fn computers(
    devices: &[Device],
    presence: &[Presence],
) -> Vec<(DeviceId, Option<DateTime<Utc>>)> {
    let on_link = |device: &Device| {
        let host: IpAddr = device.host.parse().ok()?;
        let listed = presence.iter().find(|listed| listed.address == host)?;
        Some((device.id, listed.seen))
    };
    devices.iter().filter_map(on_link).collect()
}

/// The watch of a hosted hub on the computers of its link.
pub struct Watch {
    control: PathBuf,
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    /// When the hub started to watch: its start, or the end of its last sleep.
    since: DateTime<Utc>,
    /// The time of the last pass.
    last: DateTime<Utc>,
    /// The checks reported Down since their computer was last connected.
    gone: HashSet<CheckId>,
}

/// In the hosted mode, makes each check `Connected` wait one whole interval, before the
/// scheduler can find it due, and returns the watch. `None` for a hub that is not hosted.
pub async fn start(
    config: &Config,
    db: &PgPool,
    bus: &Bus,
    clock: &Arc<dyn Clock>,
) -> Option<Watch> {
    config.hosted_cloud_key_file.as_ref()?;
    let control = config.link_control.clone()?;
    let mut watch = Watch::new(control, (db.clone(), bus.clone()), clock.clone());
    watch.begin(clock.now()).await;
    Some(watch)
}

impl Watch {
    /// A watch that has reported nothing yet, on the link at the control socket `control`.
    pub(super) fn new(control: PathBuf, (db, bus): (PgPool, Bus), clock: Arc<dyn Clock>) -> Self {
        let now = clock.now();
        Self {
            control,
            db,
            bus,
            clock,
            since: now,
            last: now,
            gone: HashSet::new(),
        }
    }

    /// Starts to watch at `now`: after a start or a sleep.
    pub(super) async fn begin(&mut self, now: DateTime<Utc>) {
        self.since = now;
        if let Err(error) = checks::own::await_reports(&self.db, OwnCheck::Connected, now).await {
            job_failed(JOB, error.fault());
        }
    }

    /// Asks the link and reports the checks, for as long as the hub runs. A link that does
    /// not answer is logged by its fault word, and nothing is reported.
    pub async fn keep(mut self) {
        let mut ticks = tokio::time::interval(EVERY);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            let now = self.clock.now();
            if now - self.last > ASLEEP {
                self.begin(now).await;
            }
            self.last = now;
            let reported = match link::presence(&self.control).await {
                Ok(presence) => self.pass(&presence, now).await,
                Err(fault) => Err(fault),
            };
            if let Err(fault) = reported {
                job_failed(JOB, fault);
            }
        }
    }

    /// One pass at `now` over what the link said.
    pub(super) async fn pass(
        &mut self,
        presence: &[Presence],
        now: DateTime<Utc>,
    ) -> Result<(), &'static str> {
        let listed = devices::list(&self.db).await.map_err(|_| "devices")?;
        for (device, seen) in computers(&listed, presence) {
            let reported = match sign(seen, now, self.since) {
                Sign::Here => self.here(device, now).await,
                Sign::Gone => self.gone(device, now).await,
                Sign::Unknown => Ok(()),
            };
            reported.map_err(|error| error.fault())?;
        }
        Ok(())
    }

    /// The computer is connected: its check is there and Up.
    async fn here(&mut self, device: DeviceId, now: DateTime<Utc>) -> Result<(), checks::Error> {
        let mut conn = self.db.acquire().await?;
        let check = checks::own::ensure(&mut conn, device, OwnCheck::Connected).await?;
        drop(conn);
        self.gone.remove(&check);
        checks::own::report((&self.db, &self.bus), check, OwnReport::Up, now).await?;
        Ok(())
    }

    /// The computer left the link: its check, if it has one, gets one Down report.
    async fn gone(&mut self, device: DeviceId, now: DateTime<Utc>) -> Result<(), checks::Error> {
        let Some(check) = checks::own::find(&self.db, device, OwnCheck::Connected).await? else {
            return Ok(());
        };
        if self.gone.insert(check) {
            let down = OwnReport::Down(GONE);
            checks::own::report((&self.db, &self.bus), check, down, now).await?;
        }
        Ok(())
    }
}
