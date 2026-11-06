//! Reads the leaf certificate an `https` origin presents, for the system-browser mode (D-9),
//! where the operator compares the fingerprint before opening the page.
//!
//! The probe connects only to the origin's own host and port, completes a TLS handshake and
//! closes: it sends no request, so there is no redirect to follow. It accepts any chain on
//! purpose (the point is to show an untrusted certificate), but still checks the handshake
//! signatures, so the server proved it holds the leaf's key.

use std::{sync::Arc, time::Duration};

use rustls::{
    ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, ring, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use super::{
    fingerprint::Fingerprint,
    pin::{Host, PinnedOrigin},
};

/// Why the leaf certificate could not be read. Carries no host, port or TLS text (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CertificateError {
    #[error("the host is not a name TLS can use")]
    Address,
    #[error("the host did not answer")]
    Unreachable,
    #[error("the TLS handshake failed")]
    Handshake,
    #[error("the server presented no certificate")]
    NoCertificate,
    #[error("the host did not finish in time")]
    Timeout,
}

impl CertificateError {
    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Address => "address",
            Self::Unreachable => "unreachable",
            Self::Handshake => "handshake",
            Self::NoCertificate => "no_certificate",
            Self::Timeout => "timeout",
        }
    }
}

/// Accepts every chain and checks the handshake signatures: see the module comment.
#[derive(Debug)]
struct AnyChain(Arc<CryptoProvider>);

impl ServerCertVerifier for AnyChain {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn connector() -> Result<TlsConnector, CertificateError> {
    let provider = Arc::new(ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|_| CertificateError::Handshake)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AnyChain(provider)))
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

fn server_name(host: &Host) -> Result<ServerName<'static>, CertificateError> {
    match host {
        Host::Name(name) => {
            ServerName::try_from(name.clone()).map_err(|_| CertificateError::Address)
        }
        Host::Address(address) => Ok(ServerName::IpAddress((*address).into())),
    }
}

/// The fingerprint of the leaf certificate the origin presents. `patience` bounds name
/// lookup, connect and handshake together.
pub async fn leaf_fingerprint(
    origin: &PinnedOrigin,
    patience: Duration,
) -> Result<Fingerprint, CertificateError> {
    let name = server_name(&origin.host)?;
    tokio::time::timeout(patience, read_leaf(origin, name))
        .await
        .map_err(|_| CertificateError::Timeout)?
}

async fn read_leaf(
    origin: &PinnedOrigin,
    name: ServerName<'static>,
) -> Result<Fingerprint, CertificateError> {
    let stream = match &origin.host {
        Host::Name(host) => TcpStream::connect((host.as_str(), origin.port)).await,
        Host::Address(address) => TcpStream::connect((*address, origin.port)).await,
    }
    .map_err(|_| CertificateError::Unreachable)?;
    let session = connector()?
        .connect(name, stream)
        .await
        .map_err(|_| CertificateError::Handshake)?;
    // Dropping the session closes the connection; no application data is ever written.
    session
        .get_ref()
        .1
        .peer_certificates()
        .and_then(<[_]>::first)
        .map(|leaf| Fingerprint::of_certificate(leaf))
        .ok_or(CertificateError::NoCertificate)
}
