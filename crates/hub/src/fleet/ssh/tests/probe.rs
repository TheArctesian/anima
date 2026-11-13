//! The probe's edges (R-032, D-5, R-034): refused targets, a closed port, a host that never
//! answers, one that does not speak SSH, and a key read with no authentication. The sign-in
//! runs against the recording server in `tests/it/sessions/host_keys`.

use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use russh::{
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server,
};
use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::fleet::{
    ssh::{SshError, Target, probe::read_host_key},
    targets::{AddressPolicy, TargetError},
    trust::PublicKey,
};

/// A server that only presents its key; every authentication is refused by default.
struct KeyOnly;

impl server::Handler for KeyOnly {
    type Error = russh::Error;
}

#[tokio::test]
async fn r_034_the_probe_returns_the_key_the_host_presents() {
    let host_key = PrivateKey::from(Ed25519Keypair::from_seed(&[3; 32]));
    let blob = host_key.public_key().to_bytes().unwrap();
    let config = Arc::new(server::Config {
        keys: vec![host_key],
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let serve = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        if let Ok(session) = server::run_stream(config, stream, KeyOnly).await {
            let _ended = session.await;
        }
    });

    let read = read_host_key(loopback(port), Duration::from_secs(5)).await;

    assert_eq!(read, Ok(PublicKey::from_blob(blob).unwrap()));
    serve.abort();
}

const SHORT: Duration = Duration::from_millis(300);

/// The hub's policy with no deny list, as the hub's `Live` holds it.
static STRICT: LazyLock<AddressPolicy> = LazyLock::new(AddressPolicy::default);
/// The tests' policy, which reaches loopback.
static LOOPBACK_FOR_TESTS: LazyLock<AddressPolicy> =
    LazyLock::new(|| AddressPolicy::default().allowing_loopback());

fn strict(host: &str, port: u16) -> Target<'_> {
    Target {
        host,
        port,
        policy: &STRICT,
    }
}

fn loopback(port: u16) -> Target<'static> {
    Target {
        host: "127.0.0.1",
        port,
        policy: &LOOPBACK_FOR_TESTS,
    }
}

#[tokio::test]
async fn r_032_the_probe_refuses_a_host_that_resolves_to_the_gateway_itself() {
    // A listener proves the refusal comes before any connection: nothing is accepted.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    for host in [
        "localhost",
        "127.0.0.1",
        "::1",
        "::ffff:127.0.0.1",
        "0.0.0.0",
    ] {
        assert_eq!(
            read_host_key(strict(host, port), SHORT).await,
            Err(SshError::RefusedTarget),
            "{host}"
        );
    }
    for host in ["169.254.169.254", "fe80::1", "224.0.0.1", "64:ff9b::7f00:1"] {
        assert_eq!(
            read_host_key(strict(host, 22), SHORT).await,
            Err(SshError::RefusedTarget),
            "{host}"
        );
    }
    let accepted = tokio::time::timeout(Duration::from_millis(50), listener.accept()).await;
    assert!(accepted.is_err(), "the probe never connected");
}

#[tokio::test]
async fn dev_013_a_closed_port_is_unreachable() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    assert_eq!(
        read_host_key(loopback(port), SHORT).await,
        Err(SshError::Unreachable)
    );
}

#[tokio::test]
async fn dev_013_a_host_that_never_answers_times_out_within_the_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let silent = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(stream);
    });
    let started = std::time::Instant::now();
    assert_eq!(
        read_host_key(loopback(port), SHORT).await,
        Err(SshError::Timeout)
    );
    assert!(started.elapsed() < SHORT * 3, "{:?}", started.elapsed());
    silent.abort();
}

#[tokio::test]
async fn dev_013_a_host_that_does_not_speak_ssh_is_a_protocol_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let http = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\nServer: banner-text\r\n\r\n")
            .await
            .unwrap();
    });
    assert_eq!(
        read_host_key(loopback(port), Duration::from_secs(5)).await,
        Err(SshError::Protocol)
    );
    http.await.unwrap();
}

#[test]
fn r_032_a_resolver_error_keeps_its_meaning_for_sessions() {
    assert_eq!(
        SshError::from(TargetError::Unresolved),
        SshError::Unresolved
    );
    assert_eq!(
        SshError::from(TargetError::Refused),
        SshError::RefusedTarget
    );
}
