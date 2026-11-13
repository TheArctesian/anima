//! Deadlines and places after the handshake (SEC-001): a connection silent after its
//! handshake, a request head without its blank line and an idle kept-alive connection are
//! closed at the head deadline; an upgraded connection outlives it and keeps its place. Every
//! step has a deadline.

use std::time::{Duration, Instant};

use axum::{Router, body::Body, extract::Request, response::Response, routing::get};
use hyper_util::rt::TokioIo;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_rustls::rustls::pki_types::ServerName;

use super::super::serve::{Limits, SERVED};
use super::edge::{DEADLINE, Door, SERVED_WITHIN, closes, door, ip};
use super::serving::{connector, peer_router};

/// The way in with a 300 ms head deadline, and a TLS client at `10.0.0.7` that sends `sent`.
async fn after_handshake(sent: &[u8]) -> (Door, tokio_rustls::client::TlsStream<TcpStream>) {
    let limits = Limits {
        header_time: Duration::from_millis(300),
        ..SERVED
    };
    let door = door(peer_router(), limits, |_| ip("10.0.0.7")).await;
    let tcp = TcpStream::connect(door.addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(&door.made).connect(name, tcp).await.unwrap();
    tls.write_all(sent).await.unwrap();
    (door, tls)
}

#[tokio::test]
async fn sec_001_a_connection_silent_after_its_handshake_is_closed() {
    let (_door, mut tls) = after_handshake(b"").await;
    let started = Instant::now();
    assert!(closes(&mut tls, SERVED_WITHIN).await);
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "not before the deadline"
    );
}

#[tokio::test]
async fn sec_001_a_request_head_without_its_blank_line_is_closed() {
    let (_door, mut tls) = after_handshake(b"GET / HTTP/1.1\r\nHost: localhost\r\n").await;
    assert!(closes(&mut tls, SERVED_WITHIN).await);
}

#[tokio::test]
async fn sec_001_an_idle_kept_alive_connection_is_closed() {
    let (_door, mut tls) = after_handshake(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await;
    let mut answer = Vec::new();
    let mut chunk = [0; 512];
    while !answer.ends_with(b"10.0.0.7") {
        let read = tokio::time::timeout(DEADLINE, tls.read(&mut chunk)).await;
        let read = read.expect("the answer in time").unwrap();
        assert_ne!(read, 0, "kept alive after the answer");
        answer.extend_from_slice(&chunk[..read]);
    }
    assert!(closes(&mut tls, SERVED_WITHIN).await);
}

/// Answers `GET /echo` by switching to a protocol that sends back four bytes.
async fn echo(mut request: Request) -> Response {
    let upgrade = hyper::upgrade::on(&mut request);
    tokio::spawn(async move {
        let mut io = TokioIo::new(upgrade.await.unwrap());
        let mut four = [0; 4];
        io.read_exact(&mut four).await.unwrap();
        io.write_all(&four).await.unwrap();
    });
    Response::builder()
        .status(101)
        .header("connection", "upgrade")
        .header("upgrade", "echo")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn sec_001_an_upgraded_connection_outlives_the_head_deadline_and_keeps_its_place() {
    let limits = Limits {
        header_time: Duration::from_millis(300),
        connections_per_peer: 1,
        ..SERVED
    };
    let door = door(Router::new().route("/echo", get(echo)), limits, |_| {
        ip("10.0.0.7")
    })
    .await;
    let tcp = TcpStream::connect(door.addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(&door.made).connect(name, tcp).await.unwrap();
    let upgrade =
        b"GET /echo HTTP/1.1\r\nHost: localhost\r\nConnection: upgrade\r\nUpgrade: echo\r\n\r\n";
    tls.write_all(upgrade).await.unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0; 1];
        let read = tokio::time::timeout(DEADLINE, tls.read_exact(&mut byte)).await;
        read.expect("the switch in time").unwrap();
        head.push(byte[0]);
    }
    assert!(
        head.starts_with(b"HTTP/1.1 101"),
        "{}",
        String::from_utf8_lossy(&head)
    );

    let mut second = TcpStream::connect(door.addr).await.unwrap();
    assert!(
        closes(&mut second, SERVED_WITHIN).await,
        "the upgraded one holds the place"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    tls.write_all(b"ping").await.unwrap();
    let mut back = [0; 4];
    let read = tokio::time::timeout(DEADLINE, tls.read_exact(&mut back)).await;
    read.expect("the echo in time").unwrap();
    assert_eq!(&back, b"ping");
}
