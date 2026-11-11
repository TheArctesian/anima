//! The live feed of session changes (starts, ends and recording stops), for the
//! `sessionChanged` subscription (RA-034, LOG-010). A change is published only once its row is
//! committed, and carries the row as committed or as read again after the commit: never
//! session content, a token or the gateway's connection ID (RA-008, LOG-007).

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

    /// Publishes a committed change to a session row other than its start, with the row as the
    /// caller wrote it or read it again after its commit: an end, or a recording stop on a row
    /// that is still live. Unlike [`Live::publish_start`], it publishes even while the hub is
    /// shutting down.
    pub(in crate::sessions) fn publish_change(&self, changed: Session) {
        self.changes.publish(changed);
    }

    /// Publishes the start of a committed live row while its ticket is still held, so every
    /// end of the session (each withdraws the ticket under the same lock first) is published
    /// after it. Returns `false`, publishing nothing, when the hub is shutting down: then
    /// `shut_down` may have withdrawn the ticket before the row was visible to it.
    ///
    /// When `endSession` withdrew the ticket between the commit and this call, or shutdown
    /// raced the commit, no start is published: the feed then carries an end with no start
    /// for that session.
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
