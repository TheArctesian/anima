//! The live feed of collections, for the `metricsAdded` subscription (D-9, D-15).

use tokio::sync::broadcast;

use super::MetricsAdded;

/// How many collections the feed holds for a slow subscriber (D-9).
const CAPACITY: usize = 256;

/// A bounded broadcast of what each collection added. A subscriber that falls more than 256
/// items behind receives `RecvError::Lagged`, skips the oldest and goes on; the collector
/// never waits for a subscriber.
#[derive(Clone, Debug)]
pub struct Feed {
    sender: broadcast::Sender<MetricsAdded>,
}

impl Feed {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self { sender }
    }

    /// Sends `added` to every current subscriber.
    pub fn publish(&self, added: MetricsAdded) {
        // `send` fails only when nobody is subscribed: a live update with no listener has
        // nothing to deliver and nothing to report.
        let _ = self.sender.send(added);
    }

    /// A receiver for every collection published from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<MetricsAdded> {
        self.sender.subscribe()
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}
