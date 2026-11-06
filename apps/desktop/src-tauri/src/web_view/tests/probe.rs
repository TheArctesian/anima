//! D-9 and WEB-008: the leaf-certificate probe against servers on 127.0.0.1 only.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rustls::{
    ServerConfig,
    crypto::ring,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::TlsAcceptor;

use crate::web_view::{CertificateError, Fingerprint, PinnedOrigin, leaf_fingerprint, pin::Host};

const PATIENCE: Duration = Duration::from_secs(5);

/// A self-signed certificate for `localhost` and 127.0.0.1, and its server config.
fn self_signed() -> (CertificateDer<'static>, Arc<ServerConfig>) {
    let names = vec!["localhost".to_owned(), "127.0.0.1".to_owned()];
    let generated = rcgen::generate_simple_self_signed(names).unwrap();
    let der = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![der.clone()], key.into())
        .unwrap();
    (der, Arc::new(config))
}

/// Serves one TLS handshake, then answers how many application bytes the client sent
/// before it closed.
async fn tls_server(config: Arc<ServerConfig>) -> (u16, JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let served = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = TlsAcceptor::from(config).accept(stream).await.unwrap();
        let mut sent = Vec::new();
        // A close without close_notify reads as an error after the bytes that did arrive.
        let _ = session.read_to_end(&mut sent).await;
        sent.len()
    });
    (port, served)
}

fn origin(url: &str) -> PinnedOrigin {
    PinnedOrigin::parse(url).unwrap()
}

#[tokio::test]
async fn d_9_the_probe_reads_the_sha_256_of_the_leaf_and_sends_no_request() {
    let (der, config) = self_signed();
    let (port, served) = tls_server(config).await;
    let url = format!("https://127.0.0.1:{port}/admin");
    let read = leaf_fingerprint(&origin(&url), PATIENCE).await.unwrap();
    assert_eq!(read, Fingerprint::of_certificate(&der));
    assert_eq!(served.await.unwrap(), 0, "no application data");
}

#[tokio::test]
async fn d_9_the_probe_looks_up_a_host_name() {
    let (der, config) = self_signed();
    let (port, _served) = tls_server(config).await;
    let url = format!("https://localhost:{port}/");
    let read = leaf_fingerprint(&origin(&url), PATIENCE).await.unwrap();
    assert_eq!(read, Fingerprint::of_certificate(&der));
}

#[tokio::test]
async fn d_9_a_server_that_never_answers_times_out_within_the_patience() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let _held = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(stream);
    });
    let started = Instant::now();
    let patience = Duration::from_millis(200);
    let url = format!("https://127.0.0.1:{port}/");
    let answer = leaf_fingerprint(&origin(&url), patience).await;
    assert_eq!(answer, Err(CertificateError::Timeout));
    let waited = started.elapsed();
    assert!(
        waited >= patience && waited < Duration::from_secs(3),
        "{waited:?}"
    );
}

#[tokio::test]
async fn d_9_a_closed_port_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = format!("https://127.0.0.1:{port}/");
    let answer = leaf_fingerprint(&origin(&url), PATIENCE).await;
    assert_eq!(answer, Err(CertificateError::Unreachable));
}

#[tokio::test]
async fn d_9_a_server_that_does_not_speak_tls_fails_the_handshake() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = stream.write_all(b"HTTP/1.0 400 Bad Request\r\n\r\n").await;
    });
    let url = format!("https://127.0.0.1:{port}/");
    let answer = leaf_fingerprint(&origin(&url), PATIENCE).await;
    assert_eq!(answer, Err(CertificateError::Handshake));
}

#[tokio::test]
async fn d_9_a_host_tls_cannot_name_is_refused_before_connecting() {
    let unnamable = PinnedOrigin {
        host: Host::Name("not a name".to_owned()),
        port: 443,
    };
    let answer = leaf_fingerprint(&unnamable, PATIENCE).await;
    assert_eq!(answer, Err(CertificateError::Address));
}

#[test]
fn log_007_certificate_errors_carry_only_a_code() {
    let codes = [
        (CertificateError::Address, "address"),
        (CertificateError::Unreachable, "unreachable"),
        (CertificateError::Handshake, "handshake"),
        (CertificateError::NoCertificate, "no-certificate"),
        (CertificateError::Timeout, "timeout"),
    ];
    for (error, code) in codes {
        assert_eq!(error.code(), code);
        assert!(!error.to_string().contains("127.0.0.1"));
    }
}
