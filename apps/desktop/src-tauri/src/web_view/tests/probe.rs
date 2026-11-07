//! D-9 and WEB-008: the leaf-certificate probe against servers on the loopback only.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rustls::{
    ServerConfig, SupportedProtocolVersion,
    crypto::ring,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
    version::{TLS12, TLS13},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::TlsAcceptor;

use crate::web_view::{CertificateError, Fingerprint, PinnedOrigin, leaf_fingerprint, pin::Host};

const PATIENCE: Duration = Duration::from_secs(5);

fn server_config(chain: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> ServerConfig {
    ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .unwrap()
}

/// A self-signed certificate for `localhost`, `127.0.0.1` and `::1`, and its server config.
fn self_signed() -> (CertificateDer<'static>, ServerConfig) {
    let names = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];
    let generated = rcgen::generate_simple_self_signed(names).unwrap();
    let der = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    (der.clone(), server_config(vec![der], key.into()))
}

/// Serves one TLS handshake on `listener`, then answers how many application bytes the
/// client sent before it closed; `None` when the handshake failed.
fn tls_server(listener: TcpListener, config: ServerConfig) -> JoinHandle<Option<usize>> {
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = TlsAcceptor::from(Arc::new(config))
            .accept(stream)
            .await
            .ok()?;
        let mut sent = Vec::new();
        // A close without close_notify reads as an error after the bytes that did arrive.
        let _ = session.read_to_end(&mut sent).await;
        Some(sent.len())
    })
}

/// A server on 127.0.0.1 and the `https` URL of it.
async fn local_server(config: ServerConfig) -> (String, JoinHandle<Option<usize>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (
        format!("https://127.0.0.1:{port}/"),
        tls_server(listener, config),
    )
}

fn origin(url: &str) -> PinnedOrigin {
    PinnedOrigin::parse(url).unwrap()
}

#[tokio::test]
async fn d_9_the_probe_reads_the_sha_256_of_the_leaf_and_sends_no_request() {
    let (der, config) = self_signed();
    let (url, served) = local_server(config).await;
    let read = leaf_fingerprint(&origin(&format!("{url}admin")), PATIENCE)
        .await
        .unwrap();
    assert_eq!(read, Fingerprint::of_certificate(&der));
    assert_eq!(served.await.unwrap(), Some(0), "no application data");
}

#[tokio::test]
async fn d_9_the_probe_looks_up_a_host_name() {
    let (der, config) = self_signed();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let _served = tls_server(listener, config);
    let url = format!("https://localhost:{port}/");
    let read = leaf_fingerprint(&origin(&url), PATIENCE).await.unwrap();
    assert_eq!(read, Fingerprint::of_certificate(&der));
}

#[tokio::test]
async fn d_9_the_probe_connects_to_an_ipv6_address() {
    // A host without an IPv6 loopback cannot run this test; it passes there without probing.
    let Ok(listener) = TcpListener::bind("[::1]:0").await else {
        return;
    };
    let port = listener.local_addr().unwrap().port();
    let (der, config) = self_signed();
    let _served = tls_server(listener, config);
    let url = format!("https://[::1]:{port}/");
    let read = leaf_fingerprint(&origin(&url), PATIENCE).await.unwrap();
    assert_eq!(read, Fingerprint::of_certificate(&der));
}

#[tokio::test]
async fn d_9_the_probe_reads_the_leaf_of_a_chain_not_its_issuer() {
    let issuer_key = rcgen::KeyPair::generate().unwrap();
    let mut issuer_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    issuer_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let issuer = issuer_params.self_signed(&issuer_key).unwrap();
    let leaf_key = rcgen::KeyPair::generate().unwrap();
    let leaf = rcgen::CertificateParams::new(vec!["127.0.0.1".to_owned()])
        .unwrap()
        .signed_by(
            &leaf_key,
            &rcgen::Issuer::from_params(&issuer_params, &issuer_key),
        )
        .unwrap();
    let chain = vec![leaf.der().clone(), issuer.der().clone()];
    let key = PrivatePkcs8KeyDer::from(leaf_key.serialize_der());
    let (url, _served) = local_server(server_config(chain, key.into())).await;
    let read = leaf_fingerprint(&origin(&url), PATIENCE).await.unwrap();
    assert_eq!(read, Fingerprint::of_certificate(leaf.der()));
}

/// Always presents one certificate and signs with one key, whatever the client asks.
#[derive(Debug)]
struct Presents(Arc<CertifiedKey>);

impl ResolvesServerCert for Presents {
    fn resolve(&self, _: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}

/// Probes a server that presents another server's certificate but signs with its own key.
async fn probe_replayed_certificate(
    version: &'static SupportedProtocolVersion,
) -> Result<Fingerprint, CertificateError> {
    let names = vec!["127.0.0.1".to_owned()];
    let victim = rcgen::generate_simple_self_signed(names.clone()).unwrap();
    let replayer = rcgen::generate_simple_self_signed(names).unwrap();
    let provider = Arc::new(ring::default_provider());
    let key = PrivatePkcs8KeyDer::from(replayer.signing_key.serialize_der());
    let signer = provider.key_provider.load_private_key(key.into()).unwrap();
    let presented = CertifiedKey::new(vec![victim.cert.der().clone()], signer);
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[version])
        .unwrap()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(Presents(Arc::new(presented))));
    let (url, _served) = local_server(config).await;
    leaf_fingerprint(&origin(&url), PATIENCE).await
}

#[tokio::test]
async fn web_008_a_tls_1_3_server_without_the_certificates_key_fails_the_handshake() {
    let answer = probe_replayed_certificate(&TLS13).await;
    assert_eq!(answer, Err(CertificateError::Handshake));
}

#[tokio::test]
async fn web_008_a_tls_1_2_server_without_the_certificates_key_fails_the_handshake() {
    let answer = probe_replayed_certificate(&TLS12).await;
    assert_eq!(answer, Err(CertificateError::Handshake));
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
        (CertificateError::NoCertificate, "no_certificate"),
        (CertificateError::Timeout, "timeout"),
    ];
    for (error, code) in codes {
        assert_eq!(error.code(), code);
        assert!(!error.to_string().contains("127.0.0.1"));
    }
}
