//! The broadcast of account changes that `sourceAccountChanged` reads: a run that starts or
//! ends, a pause, a new key, a removal. It carries the account id only; the subscription reads
//! the account as it is then.

use tokio::sync::broadcast;

use crate::ids::SourceAccountId;

/// How many changes a slow subscriber may fall behind before it skips some.
const CAPACITY: usize = 64;

/// The hub's one feed of account changes. Clones share it.
#[derive(Clone)]
pub struct Feed {
    sender: broadcast::Sender<SourceAccountId>,
}

impl Feed {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CAPACITY);
        Self { sender }
    }

    /// Tells every current subscriber that `account` changed.
    pub fn publish(&self, account: SourceAccountId) {
        // `send` fails only when nobody is subscribed: there is nobody to tell.
        let _ = self.sender.send(account);
    }

    /// A receiver for every change published from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SourceAccountId> {
        self.sender.subscribe()
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}
