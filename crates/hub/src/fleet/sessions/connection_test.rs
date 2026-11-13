//! The connection test (DEV-013, SEC-006, SEC-007): whether a device answers over SSH, RDP or
//! VNC, and the SSH host key or RDP server certificate it presents, recorded in
//! `trust::Sightings` for `acceptHostKey` and `acceptServerCertificate`.

mod desktop_test;
mod rdp;
mod reach;
mod ssh_test;
#[cfg(test)]
mod tests;
mod vnc;
mod x224;

pub(super) use desktop_test::{DesktopSettings, DesktopSubject};
pub(super) use rdp::{Reply, read_server_certificate};
pub(super) use reach::ReadError;
pub(super) use ssh_test::{ConnectionTest, TestRequest, TestSubject, Unsaved, test_connection};
pub(super) use x224::TLS_PROTOCOLS;
