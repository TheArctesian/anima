//! How fast the hub sends live requests for one account (decision 6 of the slice; GitHub
//! advises serial requests and at least a second between two writes).
//!
//! At most two live reads of an account are in flight; its changes go one at a time, each at
//! least a second after the previous one ended. A waiting request gives up after
//! [`QUEUE_WAIT`], so the queue is bounded in time. The map's lock is never held across an
//! `await`: a semaphore permit, not the lock, is what a request holds while it is sent.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::{clock::Clock, ids::SourceAccountId};

/// Live reads of one account in flight at once.
pub(super) const READS_AT_ONCE: usize = 2;
/// The least time between the end of one change of an account and the start of the next.
pub(super) const CHANGE_GAP: TimeDelta = TimeDelta::seconds(1);
/// The longest a request waits for its turn.
pub(super) const QUEUE_WAIT: Duration = Duration::from_secs(30);

/// The wait was longer than [`QUEUE_WAIT`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Busy;

struct Lanes {
    reads: Arc<Semaphore>,
    changes: Arc<Semaphore>,
    last_change: Option<DateTime<Utc>>,
}

impl Default for Lanes {
    fn default() -> Self {
        Self {
            reads: Arc::new(Semaphore::new(READS_AT_ONCE)),
            changes: Arc::new(Semaphore::new(1)),
            last_change: None,
        }
    }
}

/// The lanes of every account. Cheap to clone; clones share them.
#[derive(Clone, Default)]
pub(super) struct Pace(Arc<Mutex<HashMap<SourceAccountId, Lanes>>>);

/// The turn of one change: while it lives no other change of the account starts, and when it
/// is dropped the end of the change is noted, sent or not.
pub(super) struct ChangeTurn {
    pace: Pace,
    account: SourceAccountId,
    clock: Arc<dyn Clock>,
    _permit: OwnedSemaphorePermit,
}

impl Drop for ChangeTurn {
    fn drop(&mut self) {
        let ended = self.clock.now();
        self.pace
            .with(self.account, |lanes| lanes.last_change = Some(ended));
    }
}

/// How long a change that may start at `now` waits after a change that ended at `last`.
pub(super) fn gap(last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Duration {
    last.map_or(Duration::ZERO, |last| {
        (last + CHANGE_GAP - now).to_std().unwrap_or(Duration::ZERO)
    })
}

impl Pace {
    fn with<T>(&self, account: SourceAccountId, f: impl FnOnce(&mut Lanes) -> T) -> T {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        f(map.entry(account).or_default())
    }

    /// Waits for one of the account's read lanes; the read holds the permit until it ends.
    pub(super) async fn read(
        &self,
        account: SourceAccountId,
    ) -> Result<OwnedSemaphorePermit, Busy> {
        let lane = self.with(account, |lanes| lanes.reads.clone());
        tokio::time::timeout(QUEUE_WAIT, lane.acquire_owned())
            .await
            .map_err(|_| Busy)?
            .map_err(|_| Busy)
    }

    /// Waits for the account's change lane, then until a second has passed on `clock` since
    /// the previous change ended.
    pub(super) async fn change(
        &self,
        account: SourceAccountId,
        clock: Arc<dyn Clock>,
    ) -> Result<ChangeTurn, Busy> {
        let lane = self.with(account, |lanes| lanes.changes.clone());
        let permit = tokio::time::timeout(QUEUE_WAIT, lane.acquire_owned())
            .await
            .map_err(|_| Busy)?
            .map_err(|_| Busy)?;
        let last = self.with(account, |lanes| lanes.last_change);
        tokio::time::sleep(gap(last, clock.now())).await;
        Ok(ChangeTurn {
            pace: self.clone(),
            account,
            clock,
            _permit: permit,
        })
    }
}
