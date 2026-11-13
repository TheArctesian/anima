//! Refusals at TCP accept (NET-002, D-11): a refused peer is written to the event log once a
//! minute per address, and a list that cannot be read closes the connection and is logged once
//! a minute, not per connection. Every step has a deadline.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use sqlx::PgPool;
use tokio::net::TcpStream;

use super::super::serve::{Limits, SERVED};
use super::edge::{DEADLINE, closes, door_with, edge, ip};
use super::now;
use super::serving::peer_router;
use crate::{
    clock::FixedClock,
    fleet::access::layer::{CONNECTION, Edge, REQUEST, Verdict},
};

#[tokio::test]
async fn net_002_an_unreadable_list_closes_the_connection_before_any_tls() {
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(300))
        .connect_lazy("postgres://127.0.0.1:1/none")
        .unwrap();
    let edge = Edge::new(
        unreachable,
        crate::fleet::events::Bus::new(1),
        Arc::new(FixedClock(now())),
    );
    let limits = Limits {
        handshake_time: Duration::from_secs(60),
        ..SERVED
    };
    let door = door_with(edge, peer_router(), limits, |_| ip("10.0.0.7")).await;
    let (capture, _guard) = crate::fleet::events::process_log::tests::capture::capture_log("error");
    for _ in 0..3 {
        let mut waiting = TcpStream::connect(door.addr).await.unwrap();
        let closed = closes(&mut waiting, Duration::from_secs(5)).await;
        assert!(closed, "fail closed");
    }
    let lines = capture.lines();
    assert_eq!(lines.len(), 1, "logged once, not per connection: {lines:?}");
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

/// The `access` warnings in the event log.
async fn access_warnings(pool: &PgPool) -> Vec<String> {
    let query = "select message from events.event_log \
                 where source = 'access' and severity = 'warning' order by at, id";
    sqlx::query_scalar(query).fetch_all(pool).await.unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_d_11_a_peer_refused_at_accept_is_written_once_a_minute(pool: PgPool) {
    let permitted = ["10.0.0.0/8".parse().unwrap()];
    crate::fleet::access::permit_networks(&pool, &permitted)
        .await
        .unwrap();
    let clock = Arc::new(FixedClock(now()));
    let edge = Edge::new(pool.clone(), crate::fleet::events::Bus::new(4), clock);
    let door = door_with(edge, peer_router(), SERVED, |_| ip("192.0.2.1")).await;
    let refused =
        ["Refused a connection from 192.0.2.1: the address is not in a permitted network."];
    let mut first = TcpStream::connect(door.addr).await.unwrap();
    assert!(closes(&mut first, DEADLINE).await, "closed at accept");
    let started = Instant::now();
    while access_warnings(&pool).await.is_empty() && started.elapsed() < DEADLINE {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(access_warnings(&pool).await, refused);
    let mut second = TcpStream::connect(door.addr).await.unwrap();
    assert!(closes(&mut second, DEADLINE).await, "closed at accept");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        access_warnings(&pool).await,
        refused,
        "once a minute per address"
    );
}

#[tokio::test]
async fn net_002_an_unreadable_list_is_logged_once_a_minute_not_per_connection() {
    let (capture, _guard) = crate::fleet::events::process_log::tests::capture::capture_log("error");
    let edge = edge(&[]);
    for _ in 0..5 {
        edge.unavailable(&crate::fleet::access::Error::Unreachable(
            sqlx::Error::PoolTimedOut,
        ));
    }
    let lines = capture.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0]["fields"]["fault"], "database_unreachable",
        "{lines:?}"
    );
}
