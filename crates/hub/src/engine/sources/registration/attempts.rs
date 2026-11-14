//! The one attempt the hub holds, in memory only, and the broadcast of its changes (K-E1
//! Section 7a). No table: a hub restart drops it.
//!
//! The lock is a std mutex, never held across an `await`: each step is a closure over the
//! attempt that runs and returns under it. Each step first expires an attempt whose time is
//! over and drops one that ended more than 10 minutes ago; a change is published after the
//! lock is released.

use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::sync::{Notify, broadcast};
use uuid::Uuid;

use super::attempt::{Attempt, Refusal, Snapshot};

/// How many changes a slow subscriber may fall behind before it skips some.
const CAPACITY: usize = 32;

/// The hub's one attempt slot and its feed. Cheap to clone; clones share both.
#[derive(Clone)]
pub struct Attempts {
    held: Arc<Mutex<Option<Attempt>>>,
    sender: broadcast::Sender<Snapshot>,
    wake: Arc<Notify>,
    poll: Duration,
}

impl Attempts {
    /// An empty slot whose installation poll asks every `poll`.
    #[must_use]
    pub fn new(poll: Duration) -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self {
            held: Arc::default(),
            sender,
            wake: Arc::default(),
            poll,
        }
    }

    /// Runs `f` on the slot at `now` under the lock, after expiry, and publishes the attempt
    /// when `f` changed what may be shown.
    fn with<T>(&self, now: DateTime<Utc>, f: impl FnOnce(&mut Option<Attempt>) -> T) -> T {
        let (result, before, after) = {
            let mut slot = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            let before = slot.as_ref().map(Attempt::snapshot);
            if let Some(held) = slot.as_mut() {
                held.expire(now);
            }
            if slot.as_ref().is_some_and(|held| !held.readable(now)) {
                *slot = None;
            }
            let result = f(&mut slot);
            (result, before, slot.as_ref().map(Attempt::snapshot))
        };
        if let Some(after) = after.filter(|after| before.as_ref() != Some(after)) {
            // `send` fails only when nobody is subscribed: there is nobody to tell.
            let _ = self.sender.send(after);
        }
        result
    }

    /// Holds `attempt` in place of the one before, unless that one made an App that waits for
    /// Install.
    ///
    /// # Errors
    /// `AppWaiting` while an App waits for Install.
    pub fn start(&self, attempt: Attempt, now: DateTime<Utc>) -> Result<Snapshot, Refusal> {
        self.with(now, |slot| {
            if slot
                .as_ref()
                .is_some_and(|held| !held.ended() && held.holds_app())
            {
                return Err(Refusal::AppWaiting);
            }
            let snapshot = attempt.snapshot();
            *slot = Some(attempt);
            Ok(snapshot)
        })
    }

    /// Drops attempt `id` when it is still the one held: its start was not recorded.
    pub fn forget(&self, id: Uuid, now: DateTime<Utc>) {
        self.with(now, |slot| {
            if slot.as_ref().is_some_and(|held| held.id() == id) {
                *slot = None;
            }
        });
    }

    /// The attempt the API shows at `now`: open, or ended in the last 10 minutes.
    #[must_use]
    pub fn current(&self, now: DateTime<Utc>) -> Option<Snapshot> {
        self.with(now, |slot| slot.as_ref().map(Attempt::snapshot))
    }

    /// Runs `step` on the attempt held at `now`; on attempt `id` only, when given.
    ///
    /// # Errors
    /// `NotFound` when no attempt is held, or another one; the refusal of `step` otherwise.
    pub fn step<T>(
        &self,
        id: Option<Uuid>,
        now: DateTime<Utc>,
        step: impl FnOnce(&mut Attempt) -> Result<T, Refusal>,
    ) -> Result<T, Refusal> {
        self.with(now, |slot| match slot.as_mut() {
            Some(held) if id.is_none_or(|id| id == held.id()) => step(held),
            _ => Err(Refusal::NotFound),
        })
    }

    /// A receiver for every change from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Snapshot> {
        self.sender.subscribe()
    }

    /// Wakes the installation poll at once (a hit on `/installed`).
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    /// Resolves when [`Attempts::wake`] is called.
    pub async fn woken(&self) {
        self.wake.notified().await;
    }

    /// How long the installation poll waits between two asks.
    #[must_use]
    pub const fn poll(&self) -> Duration {
        self.poll
    }
}
