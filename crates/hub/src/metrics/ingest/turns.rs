//! Who may use the database for an agent now (K8 Section 6, MET-017).
//!
//! - At most [`ADDRESS_REQUESTS`] requests per client address until the request's credential is
//!   known, so one VPN node with junk bearers or codes cannot keep the hub busy for the others.
//! - At most [`AGENT_REQUESTS`] agent requests using the database at once, well under the pool.
//!   A request waits for its turn in order for up to [`REQUEST_WAIT`], and holds it only
//!   around its database work, never while a body arrives.
//! - One push or renewal per device, so one token cannot queue requests on its device's rows.
//!
//! A request that gets no turn is answered `UNAVAILABLE` before it waits for a connection or a
//! row. Each turn ends when its guard drops, also when the answer's future is dropped at its
//! deadline. The locks here are never held across an await.

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

/// The most requests of one client address before their credentials are known. An agent sends
/// one at a time; a site router that hides several agents behind its address still fits.
pub(super) const ADDRESS_REQUESTS: usize = 2;

/// How long a request waits for one of the [`AGENT_REQUESTS`] turns.
pub(super) const REQUEST_WAIT: Duration = Duration::from_secs(2);

/// The agent requests in progress.
#[derive(Clone)]
pub(super) struct Turns {
    requests: Arc<Semaphore>,
    addresses: InFlight<IpAddr>,
    devices: InFlight<DeviceId>,
}

impl Turns {
    /// Room for `requests` agent requests at once.
    pub(super) fn new(requests: usize) -> Self {
        Self {
            requests: Arc::new(Semaphore::new(requests)),
            addresses: InFlight::new(ADDRESS_REQUESTS),
            devices: InFlight::new(1),
        }
    }

    /// A turn to use the database, waited for in order; `None` when none came within
    /// [`REQUEST_WAIT`].
    pub(super) async fn request(&self) -> Option<OwnedSemaphorePermit> {
        let waited = Arc::clone(&self.requests).acquire_owned();
        tokio::time::timeout(REQUEST_WAIT, waited).await.ok()?.ok()
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
