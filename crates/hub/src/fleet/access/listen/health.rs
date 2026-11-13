//! `anima-hub health`, the compose healthcheck: asks the running hub's `GET /health` on its
//! listen address, over TLS when the certificate files are set, within a short deadline. It
//! sends no token and nothing secret, and logs nothing.
//!
//! With TLS the hub must show exactly the first certificate of `ANIMA_TLS_CERT_FILE`
//! (pinned): the probe connects by container address, so no name check could pass. After a
//! renewal the probe fails until the hub's hourly reload serves the new certificate.

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        self, ClientConfig, DigitallySignedStruct, SignatureScheme,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::{WebPkiSupportedAlgorithms, ring, verify_tls12_signature, verify_tls13_signature},
        pki_types::{CertificateDer, ServerName, UnixTime},
        version::TLS13,
    },
};

use super::{
    certificate::{self, Fault, TlsFiles},
    reload,
};

/// The whole probe, from connect to answer.
const DEADLINE: Duration = Duration::from_secs(5);

/// A longer answer is cut here; `ok` needs a few hundred bytes with headers.
const MAX_ANSWER: u64 = 4096;

const REQUEST: &[u8] = b"GET /health HTTP/1.1\r\nHost: hub\r\nConnection: close\r\n\r\n";

/// Why the hub is not healthy, by kind.
#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error("the TLS certificate cannot be read: {0}")]
    Certificate(#[from] Fault),
    #[error("the hub did not answer within 5 seconds")]
    Timeout,
    #[error("the hub cannot be reached ({0})")]
    Unreachable(io::ErrorKind),
    #[error("the TLS handshake failed or the hub showed another certificate")]
    Handshake,
    #[error("the hub did not answer ok")]
    Unhealthy,
    #[error("the TLS library refused the probe's settings")]
    TlsSetup,
}

/// Asks `GET /health` on `listen`; `Ok` only for status 200 with the body `ok`.
///
/// # Errors
///
/// The kind of failure; never certificate or key content.
pub async fn check_health(listen: SocketAddr, tls: Option<TlsFiles>) -> Result<(), HealthError> {
    tokio::time::timeout(DEADLINE, ask(listen, tls))
        .await
        .map_err(|_| HealthError::Timeout)?
}

async fn ask(listen: SocketAddr, tls: Option<TlsFiles>) -> Result<(), HealthError> {
    let unreachable = |error: io::Error| HealthError::Unreachable(error.kind());
    let tcp = TcpStream::connect(listen).await.map_err(unreachable)?;
    let answer = match tls {
        None => exchange(tcp).await?,
        Some(files) => {
            let connector = TlsConnector::from(pinned_config(&files).await?);
            let name = ServerName::IpAddress(listen.ip().into());
            let stream = connector
                .connect(name, tcp)
                .await
                .map_err(|_| HealthError::Handshake)?;
            exchange(stream).await?
        }
    };
    if healthy(&answer) {
        Ok(())
    } else {
        Err(HealthError::Unhealthy)
    }
}

async fn exchange(mut stream: impl AsyncRead + AsyncWrite + Unpin) -> Result<Vec<u8>, HealthError> {
    let unreachable = |error: io::Error| HealthError::Unreachable(error.kind());
    stream.write_all(REQUEST).await.map_err(unreachable)?;
    let mut answer = Vec::new();
    stream
        .take(MAX_ANSWER)
        .read_to_end(&mut answer)
        .await
        .map_err(unreachable)?;
    Ok(answer)
}

/// Whether an HTTP/1.1 answer is status 200 with the body `ok` (surrounding white space
/// allowed). Pure.
pub(super) fn healthy(answer: &[u8]) -> bool {
    let Some(split) = answer.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let (head, body) = (&answer[..split], &answer[split + 4..]);
    head.starts_with(b"HTTP/1.1 200 ") && body.trim_ascii() == b"ok"
}

/// A TLS 1.3 client that accepts only the first certificate of the hub's certificate file.
async fn pinned_config(files: &TlsFiles) -> Result<Arc<ClientConfig>, HealthError> {
    let leaf = reload::blocking(files, certificate::first_certificate).await?;
    let provider = Arc::new(ring::default_provider());
    let pinned = Pinned {
        leaf,
        algorithms: provider.signature_verification_algorithms,
    };
    let config = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(&[&TLS13])
        .map_err(|_| HealthError::TlsSetup)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(pinned))
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Trusts exactly one certificate; handshake signatures are still checked against it.
struct Pinned {
    leaf: CertificateDer<'static>,
    algorithms: WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for Pinned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pinned(..)")
    }
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if end_entity.as_ref() == self.leaf.as_ref() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "not the hub's certificate".to_owned(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, signature, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, signature, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}
