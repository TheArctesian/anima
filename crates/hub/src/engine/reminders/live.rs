//! The firings that go to the open applications (K-E6 Section 5; REM-008, REM-009). An
//! application has `Subscription.reminderFirings` open; the sender gives it each firing at the
//! send and then queues nothing to a channel.

use tokio::sync::broadcast;

use super::{
    delivery::lock_screen,
    model::{Class, Firing},
};

/// The most firings that wait for one slow subscriber; it then skips the oldest.
const CAPACITY: usize = 64;

/// The hub's one way to the open applications. `main.rs` builds it once: the schema and the
/// sender hold clones of this same value, or the sender counts no application.
#[derive(Clone, Debug)]
pub struct Live {
    sender: broadcast::Sender<Firing>,
}

impl Default for Live {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self { sender }
    }
}

impl Live {
    /// How many subscriptions are open now. A subscription that the client closed is not
    /// counted after its stream is dropped.
    #[must_use]
    pub fn open(&self) -> usize {
        self.sender.receiver_count()
    }

    /// A receiver of each firing that is sent from now on. It is one open subscription until it
    /// is dropped.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Firing> {
        self.sender.subscribe()
    }

    /// Gives `firing` to each open subscription, with the words that can be on a lock screen.
    pub(super) fn publish(&self, firing: Firing) {
        // `send` fails only when no subscription is open. The firing is in the database, so
        // nothing is lost and nothing is reported.
        let _ = self.sender.send(for_a_device(firing));
    }
}

/// The firing as it leaves the hub for the notification of a device (REM-009). For Class C: as
/// it is stored. For Class A or B: the fixed body and no subject, so that no title and no value
/// of a private record is in the payload; the application reads the firing after it is unlocked.
#[must_use]
pub fn for_a_device(firing: Firing) -> Firing {
    match firing.class {
        Class::C => firing,
        Class::A | Class::B => Firing {
            subject: String::new(),
            text: lock_screen(firing.class, "", "").1,
            ..firing
        },
    }
}
