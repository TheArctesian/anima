//! The check scheduler: one task that probes each check at its interval, records and
//! publishes the beats, reports status changes and late checks, and runs the daily jobs
//! (HUB-003, CHK-001, CHK-002, CHK-010, DB-005, DB-009; D-7).
//!
//! The decisions are pure and live in `schedule`; `tick` holds the loop state and the I/O.

mod attempt;
mod daily;
mod log;
mod notices;
mod schedule;
#[cfg(test)]
mod tests;
mod tick;

use std::{sync::Arc, time::Duration};

use rand::{SeedableRng as _, rngs::StdRng};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use crate::{clock::Clock, events::Bus};

use attempt::Context;
pub use attempt::Probing;
pub(super) use attempt::in_maintenance;
use tick::{Limits, Scheduler};

/// The time between ticks.
const TICK: Duration = Duration::from_secs(1);
/// The most probes that run at the same time (D-7), and the most due checks read per tick.
const LIMITS: Limits = Limits {
    probes: 32,
    due_per_tick: 256,
};

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
    let mut scheduler = Scheduler::new(Context { db, bus, probing }, LIMITS);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            scheduler.tick(clock.now(), &mut rng).await;
        }
    })
}
