//! `anima web accept-certificate`: pins the certificate of a web interface by its fingerprint
//! and writes an audit entry (WEB-008, WEB-009). The operator always gives the fingerprint.

use super::documents::{WebCertificateAccept, web_certificate_accept};
use super::shown::Shown;
use crate::devices::trust::fingerprint as fingerprint_argument;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct AcceptCertificateArgs {
    /// The web interface's ID (`anima web list <device>`).
    id: String,
    /// The SHA-256 fingerprint of the certificate, as the hub reported it.
    #[arg(value_parser = fingerprint_argument)]
    fingerprint: String,
}

impl AcceptCertificateArgs {
    /// Pins the certificate and prints the web interface with its pin.
    ///
    /// # Errors
    /// The hub's `CONFLICT` (exit 6) when the fingerprint is not the certificate it saw;
    /// `NOT_FOUND` for an unknown ID.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = web_certificate_accept::Variables {
            id: self.id,
            fingerprint_sha256: self.fingerprint,
        };
        let web = hub
            .run::<WebCertificateAccept>(variables)
            .await?
            .accept_web_certificate;
        invocation.output.data(&web, |web| {
            Shown {
                id: &web.id,
                label: &web.label,
                url: &web.url,
                open_in: &enum_name(&web.open_in),
                pinned_fingerprint: web.pinned_fingerprint_sha256.as_deref(),
            }
            .table()
        })
    }
}
