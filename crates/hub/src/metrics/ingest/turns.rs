//! Who may use the database for an agent now (K8 Section 6, MET-017).
//!
//! - At most [`ADDRESS_REQUESTS`] requests per client address until the request's credential is
//!   known, so one VPN node with junk bearers or codes cannot keep the hub busy for the others.
//!   All requests from one address share its turns.
//! - At most [`AGENT_REQUESTS`] agent requests using the database at once, well under the pool.
//!   A request waits for its turn in order, up to the wait it is given, and holds it only
//!   around its database work, never while a body arrives. A valid token's store holds its turn
//!   until it ends, at most until the answer's deadline.
//! - One push or renewal per device, so one token cannot queue requests on its device's rows.
//!
//! A request that gets no turn is answered `UNAVAILABLE` before it waits for a connection or a
//! row. Each turn ends when its guard drops, also when the answer's future is dropped at its
//! deadline. The locks here are never held across an await.
//!
//! A refusal's event is written after the request's turns end, at most once a minute per
//! client address, or per device when its token was valid (`throttle`).

use std::{
    collections::HashMap,
    hash::Hash,
    net::IpAddr,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::ids::DeviceId;

/// The most agent requests that use the database at once: well under the hub's pool of 10
/// connections (`PgPool::connect`'s default), so the operator's requests and the checks always
/// find one.
pub(super) const AGENT_REQUESTS: usize = 4;

/// The most requests of one client address before their credentials are known: the node's
/// agent, which sends one at a time, and an enrolment beside it (MET-012).
pub(super) const ADDRESS_REQUESTS: usize = 2;

/// The agent requests in progress.
#[derive(Clone)]
pub(super) struct Turns {
    requests: Arc<Semaphore>,
    wait: Duration,
    addresses: InFlight<IpAddr>,
    devices: InFlight<DeviceId>,
}

impl Turns {
    /// Room for `requests` agent requests at once, each waiting up to `wait` for its turn.
    pub(super) fn new(requests: usize, wait: Duration) -> Self {
        Self {
            requests: Arc::new(Semaphore::new(requests)),
            wait,
            addresses: InFlight::new(ADDRESS_REQUESTS),
            devices: InFlight::new(1),
        }
    }

    /// A turn to use the database, waited for in order; `None` when none came within the wait.
    pub(super) async fn request(&self) -> Option<OwnedSemaphorePermit> {
        let waited = Arc::clone(&self.requests).acquire_owned();
        tokio::time::timeout(self.wait, waited).await.ok()?.ok()
    }

    /// One of the address's turns; `None` while [`ADDRESS_REQUESTS`] of its requests are in
    /// progress.
    pub(super) fn address(&self, address: IpAddr) -> Option<Turn<IpAddr>> {
        self.addresses.take(address)
    }

    /// The device's one turn; `None` while another push or renewal of the device is in progress.
    pub(super) fn device(&self, device: DeviceId) -> Option<Turn<DeviceId>> {
        self.devices.take(device)
    }
}

/// How many requests of each key are in progress, up to a limit.
#[derive(Clone)]
struct InFlight<K> {
    counts: Arc<Mutex<HashMap<K, usize>>>,
    limit: usize,
}

impl<K: Copy + Eq + Hash> InFlight<K> {
    fn new(limit: usize) -> Self {
        Self {
            counts: Arc::default(),
            limit,
        }
    }

    fn take(&self, key: K) -> Option<Turn<K>> {
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        let count = counts.entry(key).or_default();
        (*count < self.limit).then(|| {
            *count += 1;
            Turn {
                key,
                counts: Arc::clone(&self.counts),
            }
        })
    }
}

/// A turn of one address or device; it ends when this drops.
pub(super) struct Turn<K: Copy + Eq + Hash> {
    key: K,
    counts: Arc<Mutex<HashMap<K, usize>>>,
}

impl<K: Copy + Eq + Hash> Drop for Turn<K> {
    fn drop(&mut self) {
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = counts.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.key);
            }
        }
    }
}
