//! The way in (NET-002, SEC-001): the peer is checked at TCP accept before any TLS work, caps
//! hold per peer and in total, and deadlines bound the handshake and every request head. The
//! accept loop here stands in for `serve::run` so that each connection can be given the peer
//! address the case needs; everything after the accept is the hub's. Every step has a deadline.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Router, body::Body, extract::Request, response::Response, routing::get};
use hyper_util::rt::TokioIo;
use ipnet::IpNet;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{TlsAcceptor, rustls::pki_types::ServerName};

use super::super::{
    certificate::{self, Current},
    serve::{Limits, SERVED, Serving},
    slots::Slots,
    tls,
};
use super::serving::{connector, get_over_tls, peer_router};
use super::{Throwaway, now, pem, provider, throwaway};
use crate::{
    access::layer::{CONNECTION, Edge, REQUEST, Verdict},
    clock::FixedClock,
};

const DEADLINE: Duration = Duration::from_secs(20);

/// How long a permitted client may take while the attack holds its connections.
const SERVED_WITHIN: Duration = Duration::from_secs(2);

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

fn edge(networks: &[&str]) -> Edge {
    let networks: Vec<IpNet> = networks.iter().map(|net| net.parse().unwrap()).collect();
    Edge::permitting(&networks, Arc::new(FixedClock(now())))
}

/// A TLS way in for `router` that permits `10.0.0.0/8`, giving the n-th connection it accepts
/// the peer `peer_of(n)`. Stops when dropped.
struct Door {
    addr: SocketAddr,
    made: Throwaway,
    accepting: JoinHandle<()>,
}

impl Drop for Door {
    fn drop(&mut self) {
        self.accepting.abort();
    }
}

async fn door(router: Router, limits: Limits, peer_of: fn(usize) -> IpAddr) -> Door {
    let made = throwaway();
    let key = certificate::certified_key(&pem(&made.cert, &made.key), &provider()).unwrap();
    let config = tls::server_config(provider(), Arc::new(Current::new(key))).unwrap();
    let acceptor = Some(TlsAcceptor::from(config));
    let serving = Serving::new(router, edge(&["10.0.0.0/8"]), acceptor, limits);
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
async fn closes<T: AsyncRead + Unpin>(io: &mut T, within: Duration) -> bool {
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
async fn attack_handshake_slots_starve_legitimate_clients_from_one_permitted_ip() {
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

#[tokio::test]
async fn attack_handshake_slots_starve_legitimate_clients_from_many_permitted_ips() {
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

#[tokio::test]
async fn d_11_a_peer_refused_at_accept_and_by_the_layer_is_written_once() {
    let edge = edge(&["10.0.0.0/8"]);
    let Verdict::Refused(at_accept) = edge.verdict(ip("192.0.2.1"), CONNECTION).await else {
        panic!("refused at accept");
    };
    assert_eq!(
        at_accept.messages(),
        ["Refused a connection from 192.0.2.1: the address is not in a permitted network."]
    );
    let Verdict::Refused(by_layer) = edge.verdict(ip("192.0.2.1"), REQUEST).await else {
        panic!("refused by the layer");
    };
    assert!(
        by_layer.messages().is_empty(),
        "one event a minute per address"
    );
    assert!(matches!(
        edge.verdict(ip("10.1.2.3"), CONNECTION).await,
        Verdict::Permitted
    ));
}
