//! What connection tests saw, as `Live` holds it: the SSH host keys and the RDP server
//! certificates each endpoint presented, for 15 minutes (SEC-006, SEC-007). Each kind has its
//! own lock, taken for one short synchronous call.

use std::sync::{MutexGuard, PoisonError};

use chrono::{DateTime, Utc};

use super::Live;
use crate::sessions::{
    public_key::PublicKey,
    trust::{Endpoint, Observation, Observations, PresentedCertificate},
};

impl Live {
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
    pub(in crate::sessions) fn observe(
        &self,
        endpoint: Endpoint,
        key: PublicKey,
        now: DateTime<Utc>,
    ) -> Observation<PublicKey> {
        self.observations().record(endpoint, key, now)
    }

    /// The key a connection test saw at `endpoint` within the last 15 minutes before `now`.
    pub(in crate::sessions) fn observed(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation<PublicKey>> {
        self.observations().find(endpoint, now)
    }

    /// Records that the RDP server at `endpoint` presented `certificate` at `now` (SEC-007).
    pub(in crate::sessions) fn observe_certificate(
        &self,
        endpoint: Endpoint,
        certificate: PresentedCertificate,
        now: DateTime<Utc>,
    ) -> Observation<PresentedCertificate> {
        self.certificates().record(endpoint, certificate, now)
    }

    /// The certificate a connection test saw at `endpoint` within the last 15 minutes before
    /// `now`.
    pub(in crate::sessions) fn observed_certificate(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation<PresentedCertificate>> {
        self.certificates().find(endpoint, now)
    }
}
