//! In-process broadcast of new events and status changes for subscriptions (LOG-010).

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

use super::model::{EventSummary, HeartbeatSummary};
use crate::{
    ids::{CheckId, DeviceId},
    status::Status,
};

/// A change that live views show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signal {
    Event(EventSummary),
    /// A device's status changed at `at` (R-028).
    DeviceStatus {
        device: DeviceId,
        status: Status,
        at: DateTime<Utc>,
    },
    Heartbeat {
        check: CheckId,
        device: DeviceId,
        beat: HeartbeatSummary,
    },
}

/// A bounded broadcast channel. A subscriber that falls more than `capacity` signals behind
/// receives `RecvError::Lagged` and skips the oldest signals; publishers never wait.
#[derive(Clone, Debug)]
pub struct Bus {
    sender: broadcast::Sender<Signal>,
}

impl Bus {
    /// # Panics
    ///
    /// If `capacity` is 0.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Sends `signal` to every current subscriber.
    pub fn publish(&self, signal: Signal) {
        // `send` fails only when nobody is subscribed. A signal is a live update, not a
        // record, so with no listener there is nothing to deliver and nothing to report.
        let _ = self.sender.send(signal);
    }

    /// A receiver for every signal published from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Signal> {
        self.sender.subscribe()
    }
}
