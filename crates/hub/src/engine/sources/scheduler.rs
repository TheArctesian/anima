//! The loop that starts due runs (SRC-021, EXT-002; K-E1 Section 10).
//!
//! Every minute it reads the due accounts, keeps those whose source has a consumer that is on,
//! and starts at most four runs at a time. The runs live in the loop's task set, so stopping
//! the loop at shutdown cancels them; a cancelled run's claim runs out after ten minutes.

use std::{collections::HashSet, time::Duration};

use tokio::{task::JoinSet, time::MissedTickBehavior};

use super::{Error, Scheduling, model::Trigger, run, schedule, store};
use crate::engine::extensions::{self, Key};

/// How often the loop looks for due accounts.
const TICK: Duration = Duration::from_secs(60);
/// The most runs at one time.
const MAX_RUNNING: usize = 4;

/// Starts the scheduler: one tick every minute, the first at once.
#[must_use]
pub fn spawn_scheduler(scheduling: Scheduling) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut running = JoinSet::new();
        loop {
            ticker.tick().await;
            while running.try_join_next().is_some() {}
            if let Err(error) = start_due(&scheduling, &mut running).await {
                // Fixed words only: the error holds no reply, key or token (LOG-007).
                tracing::warn!(%error, "source scheduling failed");
            }
        }
    })
}

/// Starts in `running` the runs that are due and fit beside those still running there, and
/// returns how many it started.
///
/// # Errors
/// `Database` or `Extensions` when the due accounts or the switches cannot be read.
pub async fn start_due(scheduling: &Scheduling, running: &mut JoinSet<()>) -> Result<usize, Error> {
    let room = MAX_RUNNING.saturating_sub(running.len());
    if room == 0 {
        return Ok(0);
    }
    let now = scheduling.clock.now();
    let due = store::due(&scheduling.pool, now).await?;
    if due.is_empty() {
        return Ok(0);
    }
    let on: HashSet<Key> = extensions::on_keys(&scheduling.pool).await?;
    let registry = &scheduling.registry;
    let consumer_on = |source: &str| registry.consumer_on(source, &on);
    let starting = schedule::runnable(&due, consumer_on, room);
    for account in &starting {
        let (scheduling, account) = (scheduling.clone(), *account);
        running.spawn(async move {
            if let Err(error) = run::run(&scheduling, account, Trigger::Schedule).await {
                tracing::warn!(%error, "source sync could not be written");
            }
        });
    }
    Ok(starting.len())
}
