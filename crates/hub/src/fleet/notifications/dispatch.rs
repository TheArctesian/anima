//! The dispatcher: one task that sends the queued notifications and retries the failed ones
//! (D-16, NOT-001, NOT-002, P3-003).
//!
//! Each tick it reads the installation's templates, claims up to [`BATCH`] due outbox rows and
//! sends them at the same time, each through `Sender` and the one egress path, then waits for
//! all of them before it claims again. The decisions are pure: the reason and permanence of a
//! failure (`failure`), the retry schedule (`retry`) and the request for each kind
//! (`outgoing`). The time comes from the clock once per tick, so a test drives the dispatcher
//! with any instant it likes (CQ-021).
//!
//! ## Leases and duplicates
//!
//! A claim moves the row's `next_attempt_at` [`LEASE`] ahead in the same statement that takes
//! it (`FOR UPDATE SKIP LOCKED`), so two dispatchers take disjoint rows, and no claimer takes a
//! row again until its lease ends. A send takes at most the egress time limit (8 s), and every
//! row of a batch is sent at once, so a batch ends far inside its lease; the delivery and the
//! row's new state are then written in one transaction.
//!
//! Delivery is at least once. A row is sent again only when its lease ends before its outcome
//! is written: the hub stopped or crashed between the send and the write, or that write kept
//! failing for the whole lease. Within a lease no row is sent twice. Each claim counts a try,
//! so those resends end with the row's tries, and a claimer whose lease ended changes nothing
//! in the row: every outcome write matches the lease end its claim set. A send whose outcome the
//! target decided is never retried for a late answer: the egress counts an answered 2xx and a
//! mail accepted after DATA as sent (S-A, S-B).
//!
//! The task runs until its handle is aborted; aborting it also aborts the sends in flight,
//! whose rows come back when their leases end.

mod claimed;
mod failure;
mod outgoing;
mod retry;
mod sender;
#[cfg(test)]
pub(super) mod tests;

use std::{sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::{task::JoinSet, time::MissedTickBehavior};

use super::{Error, Outbox, egress::Egress, store};
use crate::{
    clock::Clock,
    fleet::{
        credentials::Vault,
        events::{self, Bus},
        targets::AddressPolicy,
    },
};

use claimed::Finished;
use failure::Failure;
pub(super) use failure::delivery;
pub(super) use sender::Sender;

/// The time between ticks.
const TICK: Duration = Duration::from_secs(1);
/// How long a claimed row is kept from every other claimer: far longer than a send (8 s) and
/// its two database round trips.
const LEASE: TimeDelta = TimeDelta::minutes(5);
/// The most rows one tick claims and sends at the same time.
const BATCH: i64 = 16;

/// What the dispatcher sends with: the vault that opens channel secrets and the address
/// policy every notification connection obeys (K3, P3-003).
pub struct Sending {
    pub vault: Vault,
    pub policy: AddressPolicy,
}

/// Why a claimed row could not be finished. The row comes back when its lease ends.
#[derive(Debug, thiserror::Error)]
enum Fault {
    #[error("the notification store failed")]
    Store(#[from] Error),
    #[error("the Warning event could not be written")]
    Events(#[from] events::Error),
}

impl Fault {
    /// The fixed word the process log carries (LOG-007).
    fn word(&self) -> &'static str {
        match self {
            Self::Store(error) => error.fault(),
            Self::Events(error) => error.fault(),
        }
    }
}

/// Starts the dispatcher. It ticks about once a second at the clock's time and runs until the
/// handle is aborted. It never panics on a failed read or write: it logs a fixed word and the
/// row comes back after its lease.
#[must_use]
pub fn spawn_dispatcher(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    sending: Sending,
) -> tokio::task::JoinHandle<()> {
    let sender = Sender::new(sending.vault, Egress::new(sending.policy));
    let dispatcher = Arc::new(Dispatcher { db, bus, sender });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            dispatcher.tick(clock.now()).await;
        }
    })
}

/// The state of one dispatcher. Each row of a batch runs in its own task; none holds a lock.
struct Dispatcher {
    db: PgPool,
    bus: Bus,
    sender: Sender,
}

impl Dispatcher {
    /// One pass at `now`: claims the due rows, sends them at the same time and waits for every
    /// one. Returns how many rows it claimed.
    async fn tick(self: &Arc<Self>, now: DateTime<Utc>) -> usize {
        let defaults = match store::installation_settings(&self.db).await {
            Ok(settings) => Arc::new(settings),
            Err(error) => {
                warn_fault(None, &Fault::from(error));
                return 0;
            }
        };
        let claimed = match store::claim_due(&self.db, now, LEASE, BATCH).await {
            Ok(claimed) => claimed,
            Err(error) => {
                warn_fault(None, &Fault::from(error));
                return 0;
            }
        };
        let count = claimed.len();
        let mut sends = JoinSet::new();
        for row in claimed {
            let (dispatcher, defaults) = (Arc::clone(self), Arc::clone(&defaults));
            sends.spawn(async move {
                let finished = dispatcher.finish(&row, &defaults, now).await;
                (row, finished)
            });
        }
        while let Some(joined) = sends.join_next().await {
            match joined {
                Ok((row, finished)) => log_finished(&row, &finished),
                Err(error) => tracing::error!(
                    domain = "notifications",
                    panicked = error.is_panic(),
                    "a notification send ended abnormally; its row comes back after its lease"
                ),
            }
        }
        count
    }
}

/// The process log line of a finished row: the channel id, the reason word and the code only
/// (LOG-007). A sent or ended row writes nothing.
fn log_finished(row: &Outbox, finished: &Result<Finished, Fault>) {
    match finished {
        Ok(Finished::Sent | Finished::Ended) => {}
        Ok(Finished::OutOfTries) => tracing::info!(
            domain = "notifications",
            channel_id = %row.channel,
            attempts = row.attempts,
            "a notification ran out of tries with no outcome recorded"
        ),
        Ok(Finished::Failed(failure)) => log_failed(row, *failure),
        Err(fault) => warn_fault(Some(row), fault),
    }
}

fn log_failed(row: &Outbox, failure: Failure) {
    tracing::info!(
        domain = "notifications",
        channel_id = %row.channel,
        reason = failure.reason.as_str(),
        code = failure.code,
        attempts = row.attempts,
        "a notification was not delivered"
    );
}

fn warn_fault(row: Option<&Outbox>, fault: &Fault) {
    tracing::warn!(
        domain = "notifications",
        channel_id = row.map(|row| tracing::field::display(row.channel)),
        fault = fault.word(),
        "a notification could not be dispatched; it is tried again after its lease"
    );
}
