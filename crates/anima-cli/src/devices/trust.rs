//! `anima device trust`: accepts a host key or a certificate. The operator always gives the
//! fingerprint; nothing is accepted on trust alone (SEC-006, SEC-007).

pub mod accept_certificate;
pub mod accept_host_key;
pub mod documents;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum TrustCommand {
    /// Trust the SSH host key with this fingerprint.
    AcceptHostKey(accept_host_key::HostKeyArgs),
    /// Trust the RDP server certificate with this fingerprint.
    AcceptCertificate(accept_certificate::CertificateArgs),
}

impl TrustCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a fingerprint the hub did not see.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::AcceptHostKey(args) => args.run(invocation).await,
            Self::AcceptCertificate(args) => args.run(invocation).await,
        }
    }
}

/// A fingerprint argument: not empty and without spaces.
///
/// # Errors
/// The reason, for clap to print.
pub fn fingerprint(text: &str) -> Result<String, String> {
    if text.is_empty() || text.contains(char::is_whitespace) {
        return Err("a SHA-256 fingerprint, such as SHA256:abc...".to_owned());
    }
    Ok(text.to_owned())
}
