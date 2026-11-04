//! Deadlines after the request head (SEC-001, NET-002): a trickled request body, a route that
//! never answers and a client that stops reading its answer each free their place, so none
//! can hold a connection past its deadline without signing in. Every step has a deadline.

use std::time::{Duration, Instant};

use axum::{Router, body::Bytes, routing::get};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_rustls::{client::TlsStream, rustls::pki_types::ServerName};

use super::super::serve::{Limits, SERVED};
use super::edge::{DEADLINE, Door, SERVED_WITHIN, door, ip};
use super::serving::{connector, get_over_tls, peer_router};

/// Larger than any socket buffer, so a client that does not read stalls the hub's writes.
const BIG: usize = 64 * 1024 * 1024;

/// `GET /` as [`peer_router`], `POST /body` reading the whole body, `GET /never` that never
/// answers and `GET /big` answering [`BIG`] bytes.
fn router() -> Router {
    peer_router()
        .route(
            "/body",
            axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
        )
        .route("/never", get(std::future::pending::<&'static str>))
        .route("/big", get(|| async { vec![b'x'; BIG] }))
}

/// A way in with one place for `10.0.0.7` and `limits` otherwise.
async fn one_place(limits: Limits) -> Door {
    let limits = Limits {
        connections_per_peer: 1,
        ..limits
    };
    door(router(), limits, |_| ip("10.0.0.7")).await
}

async fn client(door: &Door, sent: &[u8]) -> TlsStream<TcpStream> {
    let tcp = TcpStream::connect(door.addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(&door.made).connect(name, tcp).await.unwrap();
    tls.write_all(sent).await.unwrap();
    tls
}

#[tokio::test]
async fn sec_001_a_trickled_body_frees_its_place_at_the_body_deadline() {
    let door = one_place(Limits {
        body_time: Duration::from_millis(500),
        ..SERVED
    })
    .await;
    let head = b"POST /body HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1000\r\n\r\n";
    let mut trickle = client(&door, head).await;
    let started = Instant::now();
    let mut answer = Vec::new();
    let read = tokio::time::timeout(DEADLINE, async {
        loop {
            // One byte every 50 ms: never idle, never done.
            if trickle.write_all(b"x").await.is_err() {
                break;
            }
            let mut chunk = [0; 512];
            let reading = tokio::time::timeout(Duration::from_millis(50), trickle.read(&mut chunk));
            match reading.await {
                Ok(Ok(0) | Err(_)) => break,
                Ok(Ok(n)) => answer.extend_from_slice(&chunk[..n]),
                Err(_) => {}
            }
        }
    });
    read.await.expect("closed in time");
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(400),
        "not before the deadline: {took:?}"
    );
    assert!(took < SERVED_WITHIN * 2, "{took:?}");
    assert!(
        !String::from_utf8_lossy(&answer).ends_with("1000"),
        "the body never completed"
    );
    tokio::time::timeout(DEADLINE, get_over_tls(door.addr, &door.made))
        .await
        .expect("the place is free again");
}

#[tokio::test]
async fn sec_001_a_route_that_does_not_answer_gets_503_at_the_answer_deadline() {
    let door = one_place(Limits {
        answer_time: Duration::from_millis(300),
        ..SERVED
    })
    .await;
    let mut tls = client(&door, b"GET /never HTTP/1.1\r\nHost: localhost\r\n\r\n").await;
    let mut answer = [0; 12];
    let read = tokio::time::timeout(SERVED_WITHIN, tls.read_exact(&mut answer)).await;
    read.expect("an answer in time").unwrap();
    assert_eq!(&answer, b"HTTP/1.1 503");
}

#[tokio::test]
async fn sec_001_a_client_that_stops_reading_frees_its_place_at_the_write_stall() {
    let door = one_place(Limits {
        write_stall: Duration::from_millis(300),
        ..SERVED
    })
    .await;
    let mut stalled = client(&door, b"GET /big HTTP/1.1\r\nHost: localhost\r\n\r\n").await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    // One place: this answer comes only once the stalled connection has given it back.
    let answer = tokio::time::timeout(DEADLINE, get_over_tls(door.addr, &door.made)).await;
    answer.expect("the stalled one gave its place back");
    let mut got = Vec::new();
    let read = tokio::time::timeout(DEADLINE, stalled.read_to_end(&mut got)).await;
    let read = read.expect("the stalled connection was closed");
    assert!(read.is_err() || got.len() < BIG, "{}", got.len());
}
