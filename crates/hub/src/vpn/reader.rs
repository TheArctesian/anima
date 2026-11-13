//! Reads the hub's VPN status each minute, publishes the picture and writes each new warning
//! once (VPN-008, VPN-010, VPN-011).

use std::{collections::HashSet, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use super::{
    Error,
    broker::{Broker, BrokerView},
    feed::Feed,
    localapi,
    model::{BrokerState, Picture},
    peers::Status,
    picture::{self, Inputs},
    store,
    warnings::{self, Warning},
};
use crate::{
    checks::{self, config::CheckType},
    clock::Clock,
    devices,
    events::{self, ActorKind, Bus, NewEvent, Source},
    ids::{DeviceId, SiteId},
};

/// How often the hub reads its VPN status.
const INTERVAL: Duration = Duration::from_secs(60);
/// How long a read waits for the broker's node list before it publishes without it, as long
/// as the `LocalAPI` read may take (BD-15).
pub(super) const BROKER_WAIT: Duration = Duration::from_secs(10);

/// What the reader works with besides the proxy address.
#[derive(Clone)]
pub struct Reading {
    pub pool: PgPool,
    pub bus: Bus,
    pub clock: Arc<dyn Clock>,
    pub feed: Feed,
    /// The hub's one broker, read on each read (BD-16); None when `ANIMA_VPN_BROKER` is unset.
    pub broker: Option<Broker>,
}

/// The broker's answer of one read; None when the broker is not set.
pub(super) type Brokered = Option<Result<BrokerView, Error>>;

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

/// Logs only a change of the broker's state, by the fault word only (LOG-007).
fn log_broker_change(previous: Option<BrokerState>, state: BrokerState, brokered: &Brokered) {
    if previous == Some(state) {
        return;
    }
    match brokered {
        Some(Err(error)) => broker_failed(error.fault()),
        Some(Ok(_)) if previous.is_some() => broker_again(),
        _ => {}
    }
}

fn broker_failed(fault: &'static str) {
    tracing::warn!(job = "vpn_reader", fault, "VPN broker read failed");
}

fn broker_again() {
    tracing::info!(job = "vpn_reader", "VPN broker read again");
}

/// The broker state and node list of one read. A busy broker (a mutation holds the turn, or
/// the read did not end within [`BROKER_WAIT`]) keeps the last picture's state and list, and
/// is `Failed` when there is none. `NotInstalled` when the socket is not there; any other
/// failure is `Failed`.
pub(super) fn broker_answer<'a>(
    brokered: &'a Brokered,
    last: Option<&'a Picture>,
) -> (BrokerState, Option<&'a BrokerView>) {
    match (brokered, last) {
        (None, _) => (BrokerState::NotSet, None),
        (Some(Ok(view)), _) => (BrokerState::Ready, Some(view)),
        (Some(Err(Error::BrokerBusy)), Some(last)) => (last.broker, last.roster.as_ref()),
        (Some(Err(Error::NotInstalled)), _) => (BrokerState::NotInstalled, None),
        (Some(Err(_)), _) => (BrokerState::Failed, None),
    }
}

/// The broker's node list, read in a task of its own that holds the turn to its end. The read
/// waits for it at most `wait`; after that this read's answer is `BrokerBusy`, and the
/// task ends by itself within the call limit.
pub(super) async fn read_broker(broker: Option<&Broker>, wait: Duration) -> Brokered {
    let broker = broker?.clone();
    let mut task = tokio::spawn(async move { broker.nodes_if_free().await });
    match tokio::time::timeout(wait, &mut task).await {
        Ok(Ok(answer)) => Some(answer),
        Ok(Err(failed)) => std::panic::resume_unwind(failed.into_panic()),
        Err(_) => Some(Err(Error::BrokerBusy)),
    }
}

/// Reads the status and the broker, builds and publishes the picture, then writes its new
/// warnings. When the status read or the build fails, the last picture is published again as
/// not fresh; a broker failure only sets the picture's broker state. A failed warning write
/// keeps the published picture; its marks are not written, so the next read tries again.
async fn read_and_publish(localapi: &str, reading: &Reading) -> Result<(), Error> {
    let now = reading.clock.now();
    let (status, brokered) = tokio::join!(
        localapi::status(localapi),
        read_broker(reading.broker.as_ref(), BROKER_WAIT)
    );
    let previous = reading.feed.latest();
    let answer = broker_answer(&brokered, previous.as_deref());
    let built = match status {
        Ok(status) => build_picture(&status, answer, &reading.pool, now).await,
        Err(error) => Err(error),
    };
    let built = match built {
        Ok(built) => built,
        Err(error) => {
            keep_stale(&reading.feed);
            return Err(error);
        }
    };
    log_broker_change(
        previous.as_ref().map(|latest| latest.broker),
        answer.0,
        &brokered,
    );
    let picture = match previous {
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

/// The picture of `status` and the broker's answer, with the devices, the sites, and the ping
/// checks of the routers.
async fn build_picture(
    status: &Status,
    (broker_state, broker): (BrokerState, Option<&BrokerView>),
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
        broker_state,
        broker,
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
                actor: ActorKind::Hub,
            };
            events::record(&mut tx, bus, now, event).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
