//! The way in (NET-002, SEC-001): the peer is checked at TCP accept before any TLS work, caps
//! hold per peer and in total, and deadlines bound the handshake and every request head. The
//! accept loop here stands in for `serve::run` so that each connection can be given the peer
//! address the case needs; everything after the accept is the hub's. Every step has a deadline.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::Router;
use ipnet::IpNet;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{TlsAcceptor, client::TlsStream, rustls::pki_types::ServerName};

use super::super::{
    certificate::{self, Current},
    serve::{Limits, SERVED, Serving},
    slots::Slots,
    tls,
};
use super::serving::{connector, get_over_tls, peer_router};
use super::{Throwaway, now, pem, provider, throwaway};
use crate::{access::layer::Edge, clock::FixedClock};

pub(super) const DEADLINE: Duration = Duration::from_secs(20);

/// How long a permitted client may take while the attack holds its connections.
pub(super) const SERVED_WITHIN: Duration = Duration::from_secs(2);

pub(super) fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

pub(super) fn edge(networks: &[&str]) -> Edge {
    let networks: Vec<IpNet> = networks.iter().map(|net| net.parse().unwrap()).collect();
    Edge::permitting(&networks, Arc::new(FixedClock(now())))
}

/// A TLS way in for `router` that permits `10.0.0.0/8`, giving the n-th connection it accepts
/// the peer `peer_of(n)`. Stops when dropped.
pub(super) struct Door {
    pub(super) addr: SocketAddr,
    pub(super) made: Throwaway,
    accepting: JoinHandle<()>,
}

impl Drop for Door {
    fn drop(&mut self) {
        self.accepting.abort();
    }
}

pub(super) async fn door(router: Router, limits: Limits, peer_of: fn(usize) -> IpAddr) -> Door {
    door_with(edge(&["10.0.0.0/8"]), router, limits, peer_of).await
}

/// As [`door`], with `edge` deciding who is let in.
pub(super) async fn door_with(
    edge: Edge,
    router: Router,
    limits: Limits,
    peer_of: fn(usize) -> IpAddr,
) -> Door {
    let made = throwaway();
    let key = certificate::certified_key(&pem(&made.cert, &made.key), &provider()).unwrap();
    let config = tls::server_config(provider(), Arc::new(Current::new(key))).unwrap();
    let acceptor = Some(TlsAcceptor::from(config));
    let serving = Serving::new(router, edge, acceptor, limits);
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let accepting = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        let mut n = 0;
        loop {
            let (stream, real) = tcp.accept().await.unwrap();
            let peer = SocketAddr::new(peer_of(n), real.port());
            if let Some(connection) = serving.admit(stream, peer) {
                connections.spawn(connection);
            }
            n += 1;
        }
    });
    Door {
        addr,
        made,
        accepting,
    }
}

/// `count` connections that never send a byte.
async fn silent(addr: SocketAddr, count: usize) -> Vec<TcpStream> {
    let mut held = Vec::with_capacity(count);
    for _ in 0..count {
        held.push(TcpStream::connect(addr).await.unwrap());
    }
    held
}

/// Whether the server closes `io` within `within`.
pub(super) async fn closes<T: AsyncRead + Unpin>(io: &mut T, within: Duration) -> bool {
    let mut sink = Vec::new();
    tokio::time::timeout(within, io.read_to_end(&mut sink))
        .await
        .is_ok()
}

/// How many of `held` the server still keeps open.
async fn still_open(held: Vec<TcpStream>) -> usize {
    let mut looks = JoinSet::new();
    for mut stream in held {
        looks.spawn(async move { !closes(&mut stream, Duration::from_millis(500)).await });
    }
    looks
        .join_all()
        .await
        .into_iter()
        .filter(|open| *open)
        .count()
}

/// Asserts that a client at `10.0.0.7` gets its answer within [`SERVED_WITHIN`].
async fn assert_served(door: &Door) {
    let started = Instant::now();
    let (answer, _) = tokio::time::timeout(DEADLINE, get_over_tls(door.addr, &door.made))
        .await
        .expect("an answer in time");
    assert!(started.elapsed() < SERVED_WITHIN, "{:?}", started.elapsed());
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(
        answer.ends_with("10.0.0.7"),
        "the admitted peer reaches the route: {answer}"
    );
}

#[tokio::test]
async fn net_002_one_peer_holds_at_most_8_handshakes_and_others_are_served() {
    let door = door(peer_router(), SERVED, |n| {
        ip(if n < 256 { "10.0.0.66" } else { "10.0.0.7" })
    })
    .await;
    let held = silent(door.addr, 256).await;
    assert_served(&door).await;
    assert_eq!(
        still_open(held).await,
        SERVED.handshakes_per_peer,
        "one peer holds at most 8 handshakes; the rest are closed"
    );
}

/// A TLS connection whose handshake the server has finished: it has answered one request on
/// it. Kept alive for as long as the value.
async fn answered(door: &Door) -> TlsStream<TcpStream> {
    let tcp = TcpStream::connect(door.addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(&door.made).connect(name, tcp).await.unwrap();
    tls.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut head = [0; 12];
    tls.read_exact(&mut head).await.unwrap();
    assert_eq!(&head, b"HTTP/1.1 200");
    tls
}

#[tokio::test]
async fn net_002_sec_001_a_finished_handshake_gives_its_place_back() {
    let door = door(peer_router(), SERVED, |_| ip("10.0.0.7")).await;
    let mut held = Vec::new();
    for _ in 0..=SERVED.handshakes_per_peer {
        let open = tokio::time::timeout(DEADLINE, answered(&door)).await;
        held.push(open.expect("a handshake place"));
    }
    assert_served(&door).await;
    drop(held);
}

#[tokio::test]
async fn net_002_many_peers_hold_handshakes_with_no_total_cap_and_others_are_served() {
    let door = door(peer_router(), SERVED, |n| match u8::try_from(n) {
        Ok(n) => IpAddr::from([10, 0, 1, n]),
        Err(_) => ip("10.0.0.7"),
    })
    .await;
    let held = silent(door.addr, 256).await;
    assert_served(&door).await;
    assert_eq!(
        still_open(held).await,
        256,
        "no global handshake cap to fill"
    );
}

#[tokio::test]
async fn net_002_refused_peers_are_closed_at_accept_before_any_tls() {
    let door = door(peer_router(), SERVED, |n| match u8::try_from(n) {
        Ok(n) if n < 128 => IpAddr::from([172, 17, 0, n]),
        Ok(n) => IpAddr::from([192, 0, 2, n]),
        Err(_) => ip("10.0.0.7"),
    })
    .await;
    let held = silent(door.addr, 256).await;
    assert_served(&door).await;
    assert_eq!(still_open(held).await, 0, "container and outside peers");
}

#[test]
fn net_002_sec_001_the_hub_serves_with_the_reviewed_limits() {
    let Limits {
        connections,
        connections_per_peer,
        handshakes_per_peer,
        handshake_time,
        header_time,
        body_time,
        answer_time,
        write_stall,
    } = SERVED;
    assert_eq!(
        (connections, connections_per_peer, handshakes_per_peer),
        (1024, 64, 8)
    );
    assert_eq!(handshake_time, Duration::from_secs(5));
    assert_eq!(header_time, Duration::from_secs(10));
    assert_eq!(body_time, Duration::from_secs(30));
    assert_eq!(answer_time, Duration::from_mins(1));
    assert_eq!(write_stall, Duration::from_secs(30));
}

#[tokio::test]
async fn sec_001_a_stalled_handshake_is_dropped_at_its_deadline() {
    let limits = Limits {
        handshake_time: Duration::from_millis(300),
        ..SERVED
    };
    let door = door(peer_router(), limits, |_| ip("10.0.0.66")).await;
    let mut stalled = TcpStream::connect(door.addr).await.unwrap();
    assert!(
        !closes(&mut stalled, Duration::from_millis(100)).await,
        "open at first"
    );
    assert!(closes(&mut stalled, SERVED_WITHIN).await);
}

#[test]
fn net_002_slots_cap_the_total_and_each_peer() {
    let slots = Slots::new(3, 2);
    let (one, two) = (ip("10.0.0.1"), ip("10.0.0.2"));
    let first = slots.take(one).unwrap();
    let second = slots.take(one).unwrap();
    assert!(slots.take(one).is_none(), "the peer's share");
    let other = slots.take(two).unwrap();
    assert!(slots.take(ip("10.0.0.3")).is_none(), "the total");
    assert_eq!(slots.held(one), (3, 2));
    drop(first);
    assert_eq!(slots.held(one), (2, 1));
    let again = slots.take(one).unwrap();
    drop((second, again, other));
    assert_eq!(slots.held(one), (0, 0));
    assert_eq!(slots.peers(), 0, "given-back peers are forgotten");
}
