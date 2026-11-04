//! TLS serving on a loopback port the system picks (SEC-001): a client that trusts the
//! throwaway certificate gets an answer and the handler sees the TCP peer; a swapped
//! certificate reaches the next handshake. Every network step has a deadline.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, extract::ConnectInfo, routing::get};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        ClientConfig, RootCertStore,
        pki_types::{CertificateDer, ServerName},
        version::TLS13,
    },
};

use super::super::{
    Listening, TlsListener,
    certificate::{self, Current},
    listen, tls,
};
use super::{Files, Throwaway, pem, provider, throwaway};
use crate::clock::{Clock, FixedClock};

const DEADLINE: Duration = Duration::from_secs(20);

/// Answers `GET /` with the client address the connection reports.
fn peer_router() -> Router {
    Router::new().route(
        "/",
        get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.ip().to_string() }),
    )
}

fn connector(trusted: &Throwaway) -> TlsConnector {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(trusted.der.clone())).unwrap();
    let config = ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

/// The response to `GET /` over TLS, and the certificate the server showed.
async fn get_over_tls(addr: SocketAddr, trusted: &Throwaway) -> (String, Vec<u8>) {
    let tcp = TcpStream::connect(addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(trusted).connect(name, tcp).await.unwrap();
    let shown = tls.get_ref().1.peer_certificates().unwrap()[0].to_vec();
    tls.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = String::new();
    tls.read_to_string(&mut response).await.unwrap();
    (response, shown)
}

#[tokio::test]
async fn sec_001_the_hub_serves_tls_with_the_certificate_files() {
    let files = Files::new();
    let made = throwaway();
    files.write(&made.cert, &made.key, 0);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(super::now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = listen(addr, Some(files.paths.clone()), clock).await.unwrap();
    assert!(matches!(listening, Listening::Tls(_)));
    let bound = listening.local_addr().unwrap();
    let server = tokio::spawn(listening.serve(peer_router()));
    // A client that never starts its handshake does not hold up the next one.
    let _stalled = TcpStream::connect(bound).await.unwrap();

    let (response, shown) = tokio::time::timeout(DEADLINE, get_over_tls(bound, &made))
        .await
        .expect("TLS answer in time");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with("127.0.0.1"), "{response}");
    assert_eq!(shown, made.der);

    let plain = tokio::time::timeout(DEADLINE, async {
        let mut tcp = TcpStream::connect(bound).await.unwrap();
        tcp.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut answer = Vec::new();
        let _ = tcp.read_to_end(&mut answer).await;
        answer
    })
    .await
    .expect("plain request closed in time");
    assert!(!plain.starts_with(b"HTTP/"), "no plain HTTP on the TLS port");
    server.abort();
}

#[tokio::test]
async fn sec_001_plain_http_is_served_on_loopback_without_certificate_files() {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(super::now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = listen(addr, None, clock).await.unwrap();
    assert!(matches!(listening, Listening::Plain(_)));
    let not_loopback: SocketAddr = "10.0.0.1:0".parse().unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(super::now()));
    assert!(matches!(
        listen(not_loopback, None, clock).await,
        Err(super::super::Error::TlsRequired(_))
    ));
}

#[tokio::test]
async fn sec_001_a_bad_certificate_stops_listen_naming_only_the_fault() {
    let files = Files::new();
    let made = throwaway();
    files.write(&made.cert, &format!("{}junk", &made.key[..40]), 0);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(super::now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let Err(error) = listen(addr, Some(files.paths.clone()), clock).await else {
        panic!("listened with a broken key");
    };
    let text = format!("{error} {error:?}");
    assert!(text.contains("key"), "{text}");
    assert!(!text.contains(&made.key[28..40]), "{text}");
}

#[tokio::test]
async fn r_021_h_2_a_swapped_certificate_reaches_the_next_handshake() {
    let (first, second) = (throwaway(), throwaway());
    let key = certificate::certified_key(&pem(&first.cert, &first.key), &provider()).unwrap();
    let current = Arc::new(Current::new(key));
    let config = tls::server_config(provider(), Arc::clone(&current) as _).unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound = tcp.local_addr().unwrap();
    let idle = tokio::spawn(std::future::pending::<()>());
    let listening = Listening::Tls(TlsListener::new(tcp, config, idle));
    let server = tokio::spawn(listening.serve(peer_router()));

    let (_, shown) = tokio::time::timeout(DEADLINE, get_over_tls(bound, &first))
        .await
        .expect("first answer in time");
    assert_eq!(shown, first.der);
    let swapped = certificate::certified_key(&pem(&second.cert, &second.key), &provider());
    current.swap(swapped.unwrap());
    let (response, shown) = tokio::time::timeout(DEADLINE, get_over_tls(bound, &second))
        .await
        .expect("second answer in time");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(shown, second.der);
    server.abort();
}
