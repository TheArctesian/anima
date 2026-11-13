//! What connection tests saw: the SSH host keys and the RDP server certificates each endpoint
//! presented, for 15 minutes (SEC-006, SEC-007). `sessions::Live` holds a clone, so
//! `testConnection` records into it, and the schema holds the same value, so
//! `acceptHostKey` and `acceptServerCertificate` read from it. Each kind has its own lock, taken
//! for one short synchronous call and never across an `.await`.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};

use super::{
    certificate::PresentedCertificate,
    observations::{Endpoint, Observation, Observations},
    public_key::PublicKey,
};

/// The host keys and certificates connection tests saw. Cheap to clone; every clone sees the
/// same state.
#[derive(Clone, Default)]
pub struct Sightings {
    /// Its own lock: a connection test never waits on a ticket redemption.
    observations: Arc<Mutex<Observations<PublicKey>>>,
    /// RDP server certificates, apart from host keys: an endpoint may present both.
    certificates: Arc<Mutex<Observations<PresentedCertificate>>>,
}

impl Sightings {
    fn observations(&self) -> MutexGuard<'_, Observations<PublicKey>> {
        self.observations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn certificates(&self) -> MutexGuard<'_, Observations<PresentedCertificate>> {
        self.certificates
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Records that `endpoint` presented `key` at `now` (D-5).
    pub(crate) fn observe(
        &self,
        endpoint: Endpoint,
        key: PublicKey,
        now: DateTime<Utc>,
    ) -> Observation<PublicKey> {
        self.observations().record(endpoint, key, now)
    }

    /// The key a connection test saw at `endpoint` within the last 15 minutes before `now`.
    pub(in crate::fleet::trust) fn observed(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation<PublicKey>> {
        self.observations().find(endpoint, now)
    }

    /// Records that the RDP server at `endpoint` presented `certificate` at `now` (SEC-007).
    pub(crate) fn observe_certificate(
        &self,
        endpoint: Endpoint,
        certificate: PresentedCertificate,
        now: DateTime<Utc>,
    ) -> Observation<PresentedCertificate> {
        self.certificates().record(endpoint, certificate, now)
    }

    /// The certificate a connection test saw at `endpoint` within the last 15 minutes before
    /// `now`.
    pub(in crate::fleet::trust) fn observed_certificate(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation<PresentedCertificate>> {
        self.certificates().find(endpoint, now)
    }
}
