//! Reading an RDP server's certificate (SEC-007, K4 §3): the X.224 exchange asking for TLS or
//! NLA, then a TLS handshake that takes whatever certificate the server presents, and the
//! connection closes. No credential is ever sent: the hub never starts `CredSSP`. I/O edge;
//! nothing here logs, and no error holds a remote text or a host name (LOG-007).
//!
//! The verifier accepts any certificate, because the operator decides by its fingerprint;
//! it still checks the handshake signatures, so a server that replays another server's
//! certificate without its private key fails the handshake.

use std::{net::IpAddr, sync::Arc, time::Duration};

use rustls::{
    ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, ring, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;

use super::{
    reach::{ReadError, reach, within},
    x224::{self, Negotiated, PROTOCOL_HYBRID, PROTOCOL_HYBRID_EX, TPKT_HEADER},
};
use crate::{devices::RdpSecurity, sessions::ssh::Target, trust::PresentedCertificate};

/// What the server answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum Reply {
    /// The leaf certificate of the TLS handshake, and the security the server chose: TLS,
    /// NLA or NLA with the early authorisation result.
    Certificate(PresentedCertificate, RdpSecurity),
    /// The server offers standard RDP security only: there is no certificate.
    Standard,
    /// The server sent an RDP Negotiation Failure: it refuses what the hub asked for.
    Refused,
}

/// Asks `target` for `requested` (a set of `x224::PROTOCOL_*` flags) and, when it picks a TLS
/// protocol, reads its certificate; all within `limit`.
///
/// # Errors
/// [`ReadError`] when the host does not resolve, resolves to a refused address (before any
/// connection), cannot be reached, does not finish within `limit`, does not answer as an RDP
/// server ([`ReadError::Unexpected`]) or fails the TLS handshake ([`ReadError::Tls`]).
pub(in crate::sessions) async fn read_server_certificate(
    target: Target<'_>,
    requested: u32,
    limit: Duration,
) -> Result<Reply, ReadError> {
    within(limit, async {
        let mut stream = reach(target).await?;
        match confirm(&mut stream, requested).await? {
            Negotiated::Standard => Ok(Reply::Standard),
            Negotiated::Failure(_) => Ok(Reply::Refused),
            Negotiated::Tls(chosen) => {
                let certificate = leaf(stream).await?;
                Ok(Reply::Certificate(certificate, chosen_security(chosen)))
            }
        }
    })
    .await
}

/// The security setting for the TLS protocol the server chose (`x224::TLS_PROTOCOLS` flags);
/// none of them is standard RDP security.
const fn chosen_security(chosen: u32) -> RdpSecurity {
    if chosen & PROTOCOL_HYBRID_EX != 0 {
        RdpSecurity::NlaExt
    } else if chosen & PROTOCOL_HYBRID != 0 {
        RdpSecurity::Nla
    } else {
        RdpSecurity::Tls
    }
}

/// Sends the Connection Request and reads the Connection Confirm, at most
/// [`x224::MAX_CONFIRM`] bytes.
async fn confirm(stream: &mut TcpStream, requested: u32) -> Result<Negotiated, ReadError> {
    let unexpected = |_| ReadError::Unexpected;
    stream
        .write_all(&x224::connection_request(requested))
        .await
        .map_err(unexpected)?;
    let mut header = [0; TPKT_HEADER];
    stream.read_exact(&mut header).await.map_err(unexpected)?;
    let mut body = vec![0; x224::confirm_length(header).map_err(|_| ReadError::Unexpected)?];
    stream.read_exact(&mut body).await.map_err(unexpected)?;
    x224::negotiated(&body, requested).map_err(|_| ReadError::Unexpected)
}

/// The TLS handshake on `stream` and the leaf certificate it presented. The connection is
/// dropped right after, so the server receives nothing beyond the handshake.
async fn leaf(stream: TcpStream) -> Result<PresentedCertificate, ReadError> {
    let peer = stream.peer_addr().map_err(|_| ReadError::Unreachable)?.ip();
    let tls = connector()?
        .connect(server_name(peer), stream)
        .await
        .map_err(|_| ReadError::Tls)?;
    let (_, session) = tls.get_ref();
    let der = session
        .peer_certificates()
        .and_then(<[_]>::first)
        .ok_or(ReadError::Tls)?;
    Ok(PresentedCertificate::from_der(der))
}

/// The connected address as the server name: rustls sends no SNI for an IP address, and the
/// verifier does not judge the name.
fn server_name(peer: IpAddr) -> ServerName<'static> {
    ServerName::IpAddress(peer.into())
}

/// TLS 1.2 and 1.3 with ring, the verifier below, no client certificate and no SNI.
fn connector() -> Result<TlsConnector, ReadError> {
    let provider = Arc::new(ring::default_provider());
    let verifier = Arc::new(AnyCertificate(provider.clone()));
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|_| ReadError::Tls)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    config.enable_sni = false;
    Ok(TlsConnector::from(Arc::new(config)))
}

/// Accepts any certificate and checks the handshake signatures against it: see the module
/// comment.
#[derive(Debug)]
struct AnyCertificate(Arc<CryptoProvider>);

impl ServerCertVerifier for AnyCertificate {
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
