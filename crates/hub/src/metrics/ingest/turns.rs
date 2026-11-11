//! Who may use the database for an agent now (K8 Section 6, MET-017): at most
//! [`AGENT_REQUESTS`] agent requests at once, and one push or renewal per device. A request past
//! either is answered `UNAVAILABLE` before it waits for a connection or a row, so no agent, and
//! no number of requests with one token, can take the pool the rest of the hub runs on.
//!
//! Each turn is held by a guard and ends when the guard drops, also when the answer's future is
//! dropped at its deadline. The locks here are never held across an await.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex, PoisonError},
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::ids::DeviceId;

/// The most agent requests that use the database at once: well under the hub's pool of 10
/// connections (`PgPool::connect`'s default), so the operator's requests and the checks always
/// find one.
pub(super) const AGENT_REQUESTS: usize = 4;

/// The agent requests in progress.
#[derive(Clone)]
pub(super) struct Turns {
    requests: Arc<Semaphore>,
    devices: Arc<Mutex<HashSet<DeviceId>>>,
}

impl Turns {
    /// Room for `requests` agent requests at once.
    pub(super) fn new(requests: usize) -> Self {
        Self {
            requests: Arc::new(Semaphore::new(requests)),
            devices: Arc::default(),
        }
    }

    /// A turn for one request to use the database; `None` while [`AGENT_REQUESTS`] are in
    /// progress.
    pub(super) fn request(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.requests).try_acquire_owned().ok()
    }

    /// The device's one turn; `None` while another push or renewal of the device is in progress.
    pub(super) fn device(&self, device: DeviceId) -> Option<DeviceTurn> {
        let mut devices = self.devices.lock().unwrap_or_else(PoisonError::into_inner);
        devices.insert(device).then(|| DeviceTurn {
            device,
            devices: Arc::clone(&self.devices),
        })
    }
}

/// A device's turn; the next request of the device may start once it drops.
pub(super) struct DeviceTurn {
    device: DeviceId,
    devices: Arc<Mutex<HashSet<DeviceId>>>,
}

impl Drop for DeviceTurn {
    fn drop(&mut self) {
        let mut devices = self.devices.lock().unwrap_or_else(PoisonError::into_inner);
        devices.remove(&self.device);
    }
}
