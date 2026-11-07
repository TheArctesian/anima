//! The host keys and RDP server certificates connection tests saw, for an accept to compare
//! with (SEC-006, SEC-007). Each has its own lock, so a test never waits on a ticket.

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
