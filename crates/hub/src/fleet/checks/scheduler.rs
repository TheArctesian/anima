//! The check scheduler: one task that probes each check at its interval, records and
//! publishes the beats, reports status changes and late checks (HUB-003, CHK-001, CHK-002, CHK-010).
//!
//! The decisions are pure and live in `schedule`; `tick` holds the loop state and the I/O.

mod attempt;
mod log;
mod moved_host;
mod notices;
mod repair;
mod schedule;
#[cfg(test)]
mod tests;
mod tick;

use std::{sync::Arc, time::Duration};

use rand::{SeedableRng as _, rngs::StdRng};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use crate::{clock::Clock, fleet::events::Bus};

use attempt::Context;
pub use attempt::Probing;
pub(super) use attempt::in_maintenance;
use tick::{Limits, Scheduler};

/// The time between ticks.
const TICK: Duration = Duration::from_secs(1);
/// The most probes that run at the same time, and the most due checks read per tick.
const LIMITS: Limits = Limits {
    probes: 32,
    due_per_tick: 256,
};

/// Starts the scheduler. It ticks about once a second at the clock's time and runs until
/// the handle is aborted. A wait for a tick that the clock says was much longer means that
/// the hub did not run (it slept, or its computer did): the scheduler then goes on as after
/// a start, so that time is not a missed push or a late check (HMD-047, CHK-010); aborting it also aborts the probes in flight. It never panics on
/// a failed read or write: it logs and tries again on a later tick.
#[must_use]
pub fn spawn_scheduler(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    probing: Probing,
) -> tokio::task::JoinHandle<()> {
    let mut rng = StdRng::from_rng(&mut rand::rng());
    let mut scheduler = Scheduler::new(Context::new(db, bus, probing), LIMITS);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut idle_since = None;
        loop {
            ticker.tick().await;
            let now = clock.now();
            if schedule::slept(idle_since, now) {
                scheduler.woke(now);
            }
            scheduler.tick(now, &mut rng).await;
            idle_since = Some(clock.now());
        }
    })
}
