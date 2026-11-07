//! Trusting what a device presents (SEC-006, SEC-007, DEV-013): the connection test, the SSH
//! host keys and RDP server certificates it saw, and the operator accepting one of them
//! (D-5, D-6).

mod accept;
mod accept_certificate;
mod certificate;
mod connection_test;
mod desktop_test;
mod observations;
mod rdp;
mod reach;
#[cfg(test)]
mod tests;
mod vnc;
mod x224;

pub(super) use accept::{Acceptance, accept_host_key};
pub(super) use accept_certificate::{CertificateAcceptance, accept_server_certificate};
pub(super) use certificate::{CertificateFingerprint, PresentedCertificate};
pub(super) use connection_test::{
    ConnectionTest, TestRequest, TestSubject, Unsaved, test_connection,
};
pub(super) use desktop_test::{Desktop, DesktopSettings, DesktopSubject};
pub(super) use observations::{Endpoint, Observation, Observations};
#[cfg(test)]
pub(super) use observations::{LIFETIME, MAX_OBSERVATIONS};
