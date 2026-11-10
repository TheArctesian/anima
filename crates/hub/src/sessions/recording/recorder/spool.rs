//! The queue shared by the relay and the writer task: a lock that is never held across an
//! `await`, and a wake-up for the writer.

use std::sync::{Mutex, MutexGuard, PoisonError};

use tokio::{sync::Notify, time::Instant};

use super::queue::{Job, Next, Pushed, Queue};
use crate::sessions::recording::model::StopReason;

pub(super) struct Spool {
    queue: Mutex<Queue>,
    wake: Notify,
}

impl Spool {
    pub(super) fn new(max_bytes: u64) -> Self {
        Self {
            queue: Mutex::new(Queue::new(max_bytes)),
            wake: Notify::new(),
        }
    }

    /// The relay's push: takes the lock briefly and never waits.
    pub(super) fn push(&self, bytes: &[u8]) {
        let pushed = self.queue().push(bytes, Instant::now());
        if matches!(pushed, Pushed::Kept { wake: true } | Pushed::Stopped(_)) {
            self.wake.notify_one();
        }
    }

    pub(super) fn close(&self) {
        self.queue().close();
        self.wake.notify_one();
    }

    pub(super) fn fail(&self, reason: StopReason) {
        self.queue().fail(reason);
    }

    /// The writer's next job; waits while there is none.
    pub(super) async fn next(&self) -> Job {
        loop {
            // The guard drops at the end of this statement, before any wait.
            let next = self.queue().next(Instant::now());
            match next {
                Next::Write(job) => return job,
                Next::Wait(None) => self.wake.notified().await,
                Next::Wait(Some(due)) => {
                    tokio::select! {
                        () = self.wake.notified() => {}
                        () = tokio::time::sleep_until(due) => {}
                    }
                }
            }
        }
    }

    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
