//! `anima-hub health` against a hub on a loopback port the system picks: `ok` over TLS with
//! the pinned certificate, and a plain failure kind for every other outcome. Every step has a
//! deadline.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, routing::get};
use tokio::net::TcpListener;

use super::super::{HealthError, Listening, check_health, health::healthy, listen};
use super::{Files, now, throwaway};
use crate::clock::{Clock, FixedClock};

const DEADLINE: Duration = Duration::from_secs(20);

#[test]
fn h_3_only_status_200_with_the_body_ok_is_healthy() {
    let cases: [(&[u8], bool); 8] = [
        (b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok", true),
        (b"HTTP/1.1 200 OK\r\n\r\nok\n", true),
        (b"HTTP/1.1 200 OK\r\n\r\nnot ok", false),
        (b"HTTP/1.1 503 Service Unavailable\r\n\r\nok", false),
        (b"HTTP/1.1 2000 OK\r\n\r\nok", false),
        (b"HTTP/1.1 200 OK\r\nok", false),
        (b"HTTP/1.0 200 OK\r\n\r\nok", false),
        (b"", false),
    ];
    for (answer, expected) in cases {
        let shown = String::from_utf8_lossy(answer);
        assert_eq!(healthy(answer), expected, "{shown}");
    }
}

/// A TLS hub from `files` answering `GET /health` with `body`; returns its address.
async fn hub(files: &Files, body: &'static str) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = listen(addr, Some(files.paths.clone()), clock)
        .await
        .unwrap();
    let bound = listening.local_addr().unwrap();
    let router = Router::new().route("/health", get(move || async move { body }));
    let server = tokio::spawn(async move {
        let _ = listening.serve(router, super::nowhere()).await;
    });
    (bound, server)
}

async fn probe(addr: SocketAddr, files: Option<&Files>) -> Result<(), HealthError> {
    let tls = files.map(|files| files.paths.clone());
    tokio::time::timeout(DEADLINE, check_health(addr, tls))
        .await
        .expect("the probe keeps its own deadline")
}

#[tokio::test]
async fn h_3_a_healthy_hub_answers_ok_over_tls_with_its_own_certificate() {
    let files = Files::single();
    let made = throwaway();
    let tls_pem = format!("{}{}", made.key, made.cert);
    files.write(&tls_pem, &tls_pem, 0);
    let (addr, server) = hub(&files, "ok").await;
    assert!(probe(addr, Some(&files)).await.is_ok());

    let other = Files::single();
    let stranger = throwaway();
    let other_pem = format!("{}{}", stranger.key, stranger.cert);
    other.write(&other_pem, &other_pem, 0);
    let pinned = probe(addr, Some(&other)).await;
    assert!(matches!(pinned, Err(HealthError::Handshake)), "{pinned:?}");
    server.abort();
}

#[tokio::test]
async fn h_3_a_hub_that_answers_anything_else_is_unhealthy() {
    let files = Files::single();
    let made = throwaway();
    let tls_pem = format!("{}{}", made.key, made.cert);
    files.write(&tls_pem, &tls_pem, 0);
    let (addr, server) = hub(&files, "starting").await;
    let answer = probe(addr, Some(&files)).await;
    assert!(matches!(answer, Err(HealthError::Unhealthy)), "{answer:?}");
    server.abort();
}

#[tokio::test]
async fn h_3_plain_http_on_loopback_without_certificate_files() {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = listen(addr, None, clock).await.unwrap();
    assert!(matches!(listening, Listening::Plain(_)));
    let bound = listening.local_addr().unwrap();
    let router = Router::new().route("/health", get(|| async { "ok" }));
    let server = tokio::spawn(async move {
        let _ = listening.serve(router, super::nowhere()).await;
    });
    assert!(probe(bound, None).await.is_ok());
    server.abort();
}

#[tokio::test]
async fn h_3_no_hub_and_a_silent_hub_fail_by_kind() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gone = closed.local_addr().unwrap();
    drop(closed);
    let refused = probe(gone, None).await;
    assert!(
        matches!(refused, Err(HealthError::Unreachable(_))),
        "{refused:?}"
    );

    let silent = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = silent.local_addr().unwrap();
    let files = Files::single();
    let made = throwaway();
    let tls_pem = format!("{}{}", made.key, made.cert);
    files.write(&tls_pem, &tls_pem, 0);
    let timed_out = probe(addr, Some(&files)).await;
    assert!(
        matches!(timed_out, Err(HealthError::Timeout)),
        "{timed_out:?}"
    );
    let shown = format!("{:?} {}", timed_out, timed_out.as_ref().unwrap_err());
    assert!(!shown.contains("PRIVATE"), "{shown}");
    drop(silent);
}
