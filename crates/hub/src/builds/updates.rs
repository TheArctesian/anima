//! The live broadcast of job changes, for the `buildJobUpdated` subscription (BLD-081).

use tokio::sync::broadcast;

use super::JobUpdate;

/// How many changes the broadcast holds for a slow subscriber.
const CAPACITY: usize = 256;

/// The most line text one update carries, so a full broadcast holds at most 16 MiB of lines
/// (256 × 64 KiB). A subscriber that sees a gap in the sequences reads it with `buildJobLog`.
pub(super) const MAX_UPDATE_LINE_BYTES: usize = 64 * 1024;

/// `update` with only its newest lines whose text fits in [`MAX_UPDATE_LINE_BYTES`].
pub(super) fn bounded(mut update: JobUpdate) -> JobUpdate {
    let mut total = 0;
    let fitting = update
        .lines
        .iter()
        .rev()
        .take_while(|line| {
            total += line.text.len();
            total <= MAX_UPDATE_LINE_BYTES
        })
        .count();
    update.lines.drain(..update.lines.len() - fitting);
    update
}

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

    /// Sends `update` to every current subscriber, with only its newest lines that fit in
    /// [`MAX_UPDATE_LINE_BYTES`].
    pub(super) fn publish(&self, update: JobUpdate) {
        // `send` fails only when nobody is subscribed: a live update with no listener has
        // nothing to deliver and nothing to report.
        let _ = self.sender.send(bounded(update));
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
