//! TLS probe: completes a handshake and reports when the leaf certificate expires (CHK-007).
//!
//! The probe accepts any certificate chain on purpose: home-lab devices usually present
//! self-signed certificates, and the check answers "is the service up and when does its
//! certificate run out", not "is the chain trusted". Only the validity dates of the leaf
//! certificate are judged, against the `now` the caller supplies.

use std::{sync::Arc, time::Instant};

use chrono::{DateTime, Utc};
use rustls::{
    ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, ring},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tokio_rustls::TlsConnector;

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
    tcp,
};

/// Accepts every certificate and signature: see the module comment.
#[derive(Debug)]
struct AcceptAnyCertificate(Arc<CryptoProvider>);

impl ServerCertVerifier for AcceptAnyCertificate {
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
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn connector() -> Result<TlsConnector, Failure> {
    let provider = Arc::new(ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|_| Failure::Handshake)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate(provider)))
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

/// The `notBefore` and `notAfter` of a DER certificate.
pub(super) fn validity(der: &[u8]) -> Result<(DateTime<Utc>, DateTime<Utc>), Failure> {
    let (_, cert) =
        x509_parser::parse_x509_certificate(der).map_err(|_| Failure::CertificateUnreadable)?;
    let at = |time: x509_parser::time::ASN1Time| {
        DateTime::from_timestamp(time.timestamp(), 0).ok_or(Failure::CertificateUnreadable)
    };
    Ok((
        at(cert.validity().not_before)?,
        at(cert.validity().not_after)?,
    ))
}

/// Fails with the certificate's expiry kept in the result when `now` is outside its dates.
fn judge(
    der: &[u8],
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, (Failure, Option<DateTime<Utc>>)> {
    let (not_before, not_after) = validity(der).map_err(|failure| (failure, None))?;
    if now > not_after {
        Err((Failure::CertificateExpired, Some(not_after)))
    } else if now < not_before {
        Err((Failure::CertificateNotYetValid, Some(not_after)))
    } else {
        Ok(not_after)
    }
}

pub(super) async fn probe(
    host: &str,
    port: u16,
    server_name: Option<&str>,
    env: &ProbeEnv,
) -> ProbeResult {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let Ok(name) = ServerName::try_from(server_name.unwrap_or(bare).to_owned()) else {
        return ProbeResult::failed(&Failure::BadConfig);
    };
    let (stream, ip, connected) = match tcp::connect(host, port, env).await {
        Ok(connection) => connection,
        Err(failure) => return ProbeResult::failed(&failure),
    };
    let started = Instant::now();
    let connector = match connector() {
        Ok(connector) => connector,
        Err(failure) => return ProbeResult::failed(&failure).via(ip),
    };
    let Ok(session) = connector.connect(name, stream).await else {
        return ProbeResult::failed(&Failure::Handshake).via(ip);
    };
    let elapsed = connected + started.elapsed();
    let leaf = session
        .get_ref()
        .1
        .peer_certificates()
        .and_then(<[_]>::first);
    match leaf.map(|der| judge(der, env.now)) {
        Some(Ok(not_after)) => ProbeResult::up(elapsed).via(ip).with_certificate(not_after),
        Some(Err((failure, not_after))) => {
            let result = ProbeResult::failed(&failure).via(ip);
            not_after.map_or(result.clone(), |at| result.with_certificate(at))
        }
        None => ProbeResult::failed(&Failure::CertificateUnreadable).via(ip),
    }
}
