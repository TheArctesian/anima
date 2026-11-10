//! The stale mark of an agent (MET-018, R-098 a): once the hub has accepted no batch from an
//! agent for more than 30 seconds, the device's metrics are stale, and the hub writes one event;
//! the next accepted batch clears the mark and writes one more (in `push`). A refused batch
//! changes neither.
//!
//! Stale says only that the agent is not reaching the hub. It writes no heartbeat and changes
//! no check, so a stale agent is never a Down check. The round takes the time as a parameter
//! (CQ-021); only [`spawn_stale_watch`] reads the clock.

use std::{collections::HashSet, sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use super::{Error, store};
use crate::{
    clock::Clock,
    devices::{self, MetricsSource},
    events::{self, Bus, NewEvent, Severity, Source},
    ids::DeviceId,
};

/// How long the hub waits for a batch before the agent is stale: three push intervals.
const STALE_AFTER: TimeDelta = TimeDelta::seconds(30);

/// The time between two rounds, so an agent is marked 30 to 35 seconds after its last batch.
const ROUND: Duration = Duration::from_secs(5);

/// Starts the stale watch: a round every 5 seconds at the clock's time, skipping the ticks a
/// slow round missed. It runs until the handle is aborted, which ends a round in flight and
/// rolls back its open transaction. A failed round is logged by its fixed text and the next
/// round tries again.
#[must_use]
pub fn spawn_stale_watch(db: PgPool, bus: Bus, clock: Arc<dyn Clock>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(ROUND);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = mark_stale(&db, &bus, clock.now()).await {
                tracing::error!(%error, "stale agents could not be marked");
            }
        }
    })
}

/// Marks every agent with no batch accepted for more than 30 seconds before `now` stale, with
/// one event for each whose device still takes its metrics from the agent; how many it marked.
/// An agent that has never pushed is never marked: it never became stale. One device that
/// cannot be marked is logged by its ID and does not keep the others from being marked.
///
/// # Errors
/// `Database`, `Devices` or `Events` when the overdue agents cannot be read, or the first
/// device's failure once every other device was tried.
pub(in crate::metrics) async fn mark_stale(
    db: &PgPool,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<usize, Error> {
    let cutoff = now - STALE_AFTER;
    let mut marked = 0;
    let mut failed = None;
    for device in store::overdue(db, cutoff).await? {
        match mark_one(db, bus, device, (cutoff, now)).await {
            Ok(true) => marked += 1,
            Ok(false) => {}
            Err(error) => {
                tracing::error!(%error, %device, "an agent could not be marked stale");
                failed.get_or_insert(error);
            }
        }
    }
    failed.map_or(Ok(marked), Err)
}

/// Marks one agent stale unless a batch came meanwhile; whether it did. The device row is taken
/// first, as a batch takes it, so the mark and a batch never cross: the later one sees the other.
async fn mark_one(
    db: &PgPool,
    bus: &Bus,
    device: DeviceId,
    (cutoff, now): (DateTime<Utc>, DateTime<Utc>),
) -> Result<bool, Error> {
    let name = match devices::get(db, device).await {
        Ok(found) => found.name,
        Err(devices::Error::NotFound(_)) => return Ok(false),
        Err(other) => return Err(Error::Devices(other)),
    };
    let mut tx = db.begin().await?;
    let source = match devices::lock_metrics_source(&mut tx, device).await {
        Ok(source) => source,
        Err(devices::Error::NotFound(_)) => return Ok(false),
        Err(other) => return Err(Error::Devices(other)),
    };
    if !store::mark_stale(&mut tx, device, (cutoff, now)).await? {
        return Ok(false);
    }
    // A device switched away from the agent is marked without an event: its metrics are not
    // the agent's any more, and the mark keeps it out of the next round.
    if source == MetricsSource::Agent {
        events::record(&mut tx, bus, now, went_stale(device, &name)).await?;
    }
    tx.commit().await?;
    Ok(true)
}

/// The one event of an agent that went stale (MET-018).
fn went_stale(device: DeviceId, name: &str) -> NewEvent {
    NewEvent {
        severity: Severity::Warning,
        source: Source::Metrics,
        device: Some((device, name.to_owned())),
        message: format!("The agent on {name} sent no metrics for 30 seconds; they are stale."),
    }
}

/// Of `devices`, those with no fresh agent: revoked, not yet pushed, or marked stale. The
/// caller asks only for devices whose metrics source is `agent` (MET-018, R-092 D-7).
///
/// # Errors
/// The database error when the query fails.
pub(in crate::metrics) async fn without_fresh_agent(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashSet<DeviceId>, sqlx::Error> {
    let stale = store::without_fresh_agent(db, devices).await?;
    Ok(stale.into_iter().collect())
}
