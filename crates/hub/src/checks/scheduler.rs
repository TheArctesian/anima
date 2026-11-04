//! The check scheduler: one task that probes each check at its interval, records and
//! publishes the beats, reports status changes and late checks, and runs the daily jobs
//! (HUB-003, CHK-001, CHK-002, CHK-010, DB-005, DB-009; D-7).
//!
//! The decisions are pure and live in `schedule`; `tick` holds the loop state and the I/O.

mod attempt;
mod daily;
mod schedule;
#[cfg(test)]
mod tests;
mod tick;

use std::{sync::Arc, time::Duration};

use rand::{SeedableRng as _, rngs::StdRng};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use crate::{checks::probe::AddressPolicy, clock::Clock, credentials::Vault, events::Bus};

use attempt::Context;
use tick::Scheduler;

/// The time between ticks.
const TICK: Duration = Duration::from_secs(1);
/// The most probes that run at the same time (D-7).
const CONCURRENCY: usize = 32;

/// What probes need: the vault that opens HTTP secrets and the addresses they may reach
/// (R-030). The policy always refuses loopback, link-local and multicast addresses; the
/// deny list only refuses more (D-8).
pub struct Probing {
    pub vault: Vault,
    pub policy: AddressPolicy,
}

/// Starts the scheduler. It ticks about once a second at the clock's time and runs until
/// the handle is aborted; aborting it also aborts the probes in flight. It never panics on
/// a failed read or write: it logs and tries again on a later tick.
#[must_use]
pub fn spawn_scheduler(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    probing: Probing,
) -> tokio::task::JoinHandle<()> {
    let mut rng = StdRng::from_rng(&mut rand::rng());
    let mut scheduler = Scheduler::new(Context { db, bus, probing }, CONCURRENCY);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            scheduler.tick(clock.now(), &mut rng).await;
        }
    })
}
