//! Reads the hub's VPN status each minute, publishes the picture and writes each new warning
//! once (VPN-008, VPN-010, VPN-011).

use std::{collections::HashSet, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use super::{
    Error,
    feed::Feed,
    localapi,
    model::Picture,
    peers::Status,
    picture::{self, Inputs},
    store,
    warnings::{self, Warning},
};
use crate::{
    checks::{self, config::CheckType},
    clock::Clock,
    devices,
    events::{self, Bus, NewEvent, Source},
    ids::{DeviceId, SiteId},
};

/// How often the hub reads its VPN status.
const INTERVAL: Duration = Duration::from_secs(60);

/// What the reader works with besides the proxy address.
#[derive(Clone)]
pub struct Reading {
    pub pool: PgPool,
    pub bus: Bus,
    pub clock: Arc<dyn Clock>,
    pub feed: Feed,
}

/// Reads the hub's VPN status each 60 s and publishes the picture (VPN-008); None when the
/// `LocalAPI` proxy is not set (the vpn query then answers null). The first read is at once.
#[must_use]
pub fn spawn_reader(
    localapi: Option<String>,
    reading: Reading,
) -> Option<tokio::task::JoinHandle<()>> {
    let localapi = localapi?;
    Some(tokio::spawn(async move {
        let mut ticks = tokio::time::interval(INTERVAL);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut failed = false;
        loop {
            ticks.tick().await;
            read_once(&localapi, &reading, &mut failed).await;
        }
    }))
}

/// One read. A failure keeps the last picture, marked not fresh.
pub(super) async fn read_once(localapi: &str, reading: &Reading, last_failed: &mut bool) {
    let outcome = read_and_publish(localapi, reading).await;
    log_change(outcome.as_ref().err(), *last_failed);
    *last_failed = outcome.is_err();
}

/// Logs only a change between failing and working, by the fault word only (LOG-007): an
/// outage writes two lines, not one a minute.
fn log_change(failure: Option<&Error>, last_failed: bool) {
    match (failure, last_failed) {
        (None, true) => read_again(),
        (Some(error), false) => read_failed(error.fault()),
        _ => {}
    }
}

fn read_failed(fault: &'static str) {
    tracing::warn!(job = "vpn_reader", fault, "VPN status read failed");
}

fn read_again() {
    tracing::info!(job = "vpn_reader", "VPN status read again");
}

/// Reads, builds and publishes the picture, then writes its new warnings. When the read or
/// the build fails, the last picture is published again as not fresh. A failed warning write
/// keeps the published picture; its marks are not written, so the next read tries again.
async fn read_and_publish(localapi: &str, reading: &Reading) -> Result<(), Error> {
    let now = reading.clock.now();
    let built = match localapi::status(localapi).await {
        Ok(status) => build_picture(&status, &reading.pool, now).await,
        Err(error) => Err(error),
    };
    let built = match built {
        Ok(built) => built,
        Err(error) => {
            keep_stale(&reading.feed);
            return Err(error);
        }
    };
    let picture = match reading.feed.latest() {
        Some(previous) => picture::carry(&previous, built),
        None => built,
    };
    let due = warnings::due(&picture, now);
    reading.feed.publish(picture);
    write_warnings(&reading.pool, &reading.bus, &due, now).await
}

/// Publishes the latest picture again, marked not fresh, with its old `read_at`. A picture
/// that is already not fresh is left as it is, so subscribers are not woken for nothing.
fn keep_stale(feed: &Feed) {
    if let Some(latest) = feed.latest().filter(|latest| latest.fresh) {
        feed.publish(Picture {
            fresh: false,
            ..Picture::clone(&latest)
        });
    }
}

/// The picture of `status` with the devices, the sites, and the ping checks of the routers.
pub(super) async fn build_picture(
    status: &Status,
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<Picture, Error> {
    let devices = devices::list(pool).await?;
    let sites: Vec<(SiteId, IpNet)> = devices::sites(pool)
        .await?
        .into_iter()
        .map(|site| (site.id, site.subnet))
        .collect();
    let routers = picture::router_devices(status, &sites, &devices);
    let ping_checked: HashSet<DeviceId> = checks::for_devices(pool, &routers)
        .await?
        .into_values()
        .flatten()
        .filter(|check| check.config.kind() == CheckType::TailscalePing && !check.paused)
        .map(|check| check.device)
        .collect();
    picture::build(&Inputs {
        status,
        sites: &sites,
        devices: &devices,
        ping_checked: &ping_checked,
        now,
    })
}

/// Writes each warning whose mark is new as one `vpn` event, all in one transaction.
pub(super) async fn write_warnings(
    pool: &PgPool,
    bus: &Bus,
    warnings: &[Warning],
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    for warning in warnings {
        if store::mark(&mut tx, warning, now).await? {
            let event = NewEvent {
                severity: warning.severity,
                source: Source::Vpn,
                device: warning.device.clone(),
                message: warning.message.clone(),
            };
            events::record(&mut tx, bus, now, event).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
