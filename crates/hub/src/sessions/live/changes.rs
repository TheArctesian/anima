//! The live feed of session starts and ends, for the `sessionChanged` subscription (RA-034,
//! LOG-010). A change is published only once its row is committed, and carries the row as
//! written: never session content, a token or the gateway's connection ID (RA-008, LOG-007).

use tokio::sync::broadcast;

use super::Live;
use crate::sessions::model::{Session, SessionChange};

/// How many changes the feed holds for a slow subscriber.
const CAPACITY: usize = 256;

/// A bounded broadcast of session changes. A subscriber that falls more than 256 changes
/// behind receives `RecvError::Lagged`, skips the oldest and goes on; a publisher never waits
/// for a subscriber.
#[derive(Clone, Debug)]
pub(super) struct Changes {
    sender: broadcast::Sender<SessionChange>,
}

impl Default for Changes {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self { sender }
    }
}

impl Changes {
    /// Sends `session` to every current subscriber.
    fn publish(&self, session: Session) {
        // `send` fails only when nobody is subscribed: a change with no listener has nothing
        // to deliver and nothing to report.
        let _ = self.sender.send(SessionChange { session });
    }
}

impl Live {
    /// A receiver for every session change published from now on.
    #[must_use]
    pub fn changes(&self) -> broadcast::Receiver<SessionChange> {
        self.changes.sender.subscribe()
    }

    /// Publishes a committed end, with the row as it was written.
    pub(in crate::sessions) fn publish_end(&self, ended: Session) {
        self.changes.publish(ended);
    }

    /// Publishes the start of a committed live row while its ticket is still held, so every
    /// end of the session (each withdraws the ticket under the same lock first) is published
    /// after it. Returns `false`, publishing nothing, when the hub is shutting down: then
    /// `shut_down` may have withdrawn the ticket before the row was visible to it.
    pub(in crate::sessions) fn publish_start(&self, started: Session) -> bool {
        let state = self.state();
        if state.shutting_down {
            return false;
        }
        if state.holds_ticket(started.id) {
            // A broadcast send never waits, so it may happen under the lock.
            self.changes.publish(started);
        }
        true
    }
}
