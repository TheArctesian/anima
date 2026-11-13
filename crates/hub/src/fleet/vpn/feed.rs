//! The latest VPN picture, and a wake-up for each new one (BD-2).

use std::sync::Arc;

use tokio::sync::watch;

use super::model::Picture;

/// The hub's one VPN picture: the reader publishes on it, the `vpn` query and `vpnChanged`
/// read it. Clones share the same picture.
#[derive(Clone, Debug)]
pub struct Feed {
    sender: Arc<watch::Sender<Option<Arc<Picture>>>>,
}

impl Feed {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = watch::channel(None);
        Self {
            sender: Arc::new(sender),
        }
    }

    /// The latest picture; None before the first read or when the proxy is not set.
    #[must_use]
    pub fn latest(&self) -> Option<Arc<Picture>> {
        self.sender.borrow().clone()
    }

    /// Stores `picture` as the latest and wakes every subscriber. `send_replace` stores it
    /// also when no subscriber exists, which `send` would not.
    pub(super) fn publish(&self, picture: Picture) {
        self.sender.send_replace(Some(Arc::new(picture)));
    }

    /// A receiver that sees each picture published after this call.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Option<Arc<Picture>>> {
        self.sender.subscribe()
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}
