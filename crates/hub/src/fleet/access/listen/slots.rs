//! Connections or handshakes in progress, counted in total and per peer address, so neither
//! many clients nor one client can take every place (NET-002, SEC-001).

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
};

use crate::fleet::access::lock;

/// The places in use and the caps on them.
#[derive(Debug)]
pub(super) struct Slots {
    held: Mutex<Held>,
    total: usize,
    per_peer: usize,
}

#[derive(Debug, Default)]
struct Held {
    total: usize,
    by_peer: HashMap<IpAddr, usize>,
}

/// One place, given back when dropped.
#[derive(Debug)]
pub(super) struct Slot {
    slots: Arc<Slots>,
    peer: IpAddr,
}

impl Slots {
    /// At most `total` places at once, and at most `per_peer` for one address.
    pub(super) fn new(total: usize, per_peer: usize) -> Arc<Self> {
        Arc::new(Self {
            held: Mutex::default(),
            total,
            per_peer,
        })
    }

    /// A place for `peer`, or `None` when the total or that address's share is taken.
    pub(super) fn take(self: &Arc<Self>, peer: IpAddr) -> Option<Slot> {
        let mut held = lock(&self.held);
        let mine = held.by_peer.get(&peer).copied().unwrap_or(0);
        if held.total >= self.total || mine >= self.per_peer {
            return None;
        }
        held.total += 1;
        held.by_peer.insert(peer, mine + 1);
        Some(Slot {
            slots: Arc::clone(self),
            peer,
        })
    }

    /// Places in use, in total and for `peer`.
    #[cfg(test)]
    pub(super) fn held(&self, peer: IpAddr) -> (usize, usize) {
        let held = lock(&self.held);
        (held.total, held.by_peer.get(&peer).copied().unwrap_or(0))
    }

    /// Addresses holding a place, so the map stays as small as the connections.
    #[cfg(test)]
    pub(super) fn peers(&self) -> usize {
        lock(&self.held).by_peer.len()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut held = lock(&self.slots.held);
        held.total = held.total.saturating_sub(1);
        if let Some(mine) = held.by_peer.get_mut(&self.peer) {
            *mine = mine.saturating_sub(1);
            if *mine == 0 {
                held.by_peer.remove(&self.peer);
            }
        }
    }
}
