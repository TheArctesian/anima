//! The live broadcast of job changes, for the `buildJobUpdated` subscription (BLD-081).

use tokio::sync::broadcast;

use super::JobUpdate;

/// How many changes the broadcast holds for a slow subscriber.
const CAPACITY: usize = 256;

/// A bounded broadcast of each job change and the log lines it added. A subscriber that falls
/// more than 256 changes behind skips the oldest and goes on; a writer never waits for a
/// subscriber.
#[derive(Clone, Debug)]
pub struct Updates {
    sender: broadcast::Sender<JobUpdate>,
}

impl Updates {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self { sender }
    }

    /// Sends `update` to every current subscriber.
    pub(super) fn publish(&self, update: JobUpdate) {
        // `send` fails only when nobody is subscribed: a live update with no listener has
        // nothing to deliver and nothing to report.
        let _ = self.sender.send(update);
    }

    /// A receiver for every change published from now on.
    pub(super) fn subscribe(&self) -> broadcast::Receiver<JobUpdate> {
        self.sender.subscribe()
    }
}

impl Default for Updates {
    fn default() -> Self {
        Self::new()
    }
}
