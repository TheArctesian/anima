//! The metrics collector: every 30 seconds a round that reads each SSH metrics device over the
//! one SSH link with fixed read-only commands, stores the samples and publishes them on the
//! feed (HUB-004, MET-001, MET-002, P2-003, D-2, D-9 to D-11).
//!
//! The decisions are pure: `plan` picks the devices, `outputs` turns command output into
//! samples, `gpus` decides when to look for GPUs and makes their samples (MET-006), `failures`
//! decides the events. `collection`, `fan_out`, `round` and the GPU reads hold the I/O.

mod collection;
mod failures;
mod fan_out;
mod gpus;
mod outputs;
mod plan;
mod round;
#[cfg(test)]
mod tests;

use std::{sync::Arc, time::Duration};

use sqlx::PgPool;
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use super::Feed;
use crate::{clock::Clock, credentials, events::Bus, targets::AddressPolicy};

use round::Collector;

/// The time between rounds (MET-001).
const ROUND: Duration = Duration::from_secs(30);
/// The most collections that run at the same time (D-11).
const AT_ONCE: usize = 8;
/// The longest one device's link and commands take together (D-11).
const DEADLINE: Duration = Duration::from_secs(20);
/// How long after a collection starts its GPU reads may still run: 2 seconds inside
/// [`DEADLINE`], so a slow GPU tool ends the GPU reads, never the whole collection.
const GPU_WINDOW: Duration = Duration::from_secs(18);

/// What a collection needs to reach devices: the credential vault and the address policy
/// every link obeys (P2-004).
pub struct Collecting {
    pub vault: credentials::Vault,
    pub policy: AddressPolicy,
}

/// What every collection of the collector shares.
struct Context {
    db: PgPool,
    feed: Feed,
    bus: Bus,
    clock: Arc<dyn Clock>,
    collecting: Collecting,
}

/// Starts the collector. A round starts every 30 seconds at the clock's time; a round that
/// takes longer makes the collector skip the ticks it missed, so rounds never stack. It runs
/// until the handle is aborted, which also aborts the collections in flight and drops their
/// links. It never panics on a failed link, read or write: it logs, and the device is tried
/// again next round.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the hub contract (K3) fixes the signature of spawn_collector"
)]
pub fn spawn_collector(
    db: PgPool,
    feed: Feed,
    bus: Bus,
    clock: Arc<dyn Clock>,
    collecting: Collecting,
) -> JoinHandle<()> {
    let mut collector = Collector::new(Context {
        db,
        feed,
        bus,
        clock,
        collecting,
    });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(ROUND);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            collector.round().await;
        }
    })
}
