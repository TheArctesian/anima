//! `anima device trust accept-certificate`: trusts the RDP server certificate with this
//! fingerprint, and no other (SEC-007).

use super::documents::{CertificateAccept, certificate_accept};
use super::fingerprint;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct CertificateArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The SHA-256 fingerprint of the certificate, as `anima device test --protocol rdp` shows it.
    #[arg(value_parser = fingerprint)]
    fingerprint: String,
}

impl CertificateArgs {
    /// Trusts the certificate and prints what was accepted.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's `CONFLICT` (exit 6) when the
    /// fingerprint is not the certificate the hub saw.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = certificate_accept::Variables {
            device_id: resolve(&hub, &self.device).await?,
            fingerprint_sha256: self.fingerprint,
        };
        let certificate = hub
            .run::<CertificateAccept>(variables)
            .await?
            .accept_server_certificate;
        invocation.output.note(&format!(
            "Accepted the server certificate of {}.",
            self.device
        ));
        invocation.output.data(&certificate, |certificate| {
            let mut table = Table::record();
            table.pair("Fingerprint", certificate.fingerprint_sha256.as_str());
            table.pair("Subject", certificate.subject.as_deref().unwrap_or("-"));
            table.pair("Not after", certificate.not_after.as_deref().unwrap_or("-"));
            table.pair("First seen", certificate.first_seen_at.as_str());
            table
        })
    }
}
