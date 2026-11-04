//! The session task owns the ticket from redeem to end (MV-003, SEC-002): a client that
//! leaves during the opening, an `endSession` during the opening, and a task that panics
//! each end the session once.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::{io::AsyncWriteExt as _, net::TcpStream, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    super::super::record::Unfinished,
    Bench,
    bench::{LOOPBACK, PASSWORD, Setup, address, bench, now, operator},
    ended_once,
    fake_guacd::{ARGS_1_6_0, Act, FakeGuacd, Script, fake_guacd},
    records::ended,
    socket::open_with,
};
use crate::sessions::{Actor, end::end_session};

/// A guacd that takes `select` and then says nothing until told, and a bench in front of it.
async fn stalled(pool: &PgPool, setup: Setup) -> (FakeGuacd, Bench) {
    let guacd = fake_guacd(Script::StallBeforeArgs { args: ARGS_1_6_0 }).await;
    let bench = bench(pool, &guacd.address, setup).await;
    (guacd, bench)
}

/// Waits until the hub has sent guacd `select`: the opening is under way.
async fn opening_reached_guacd(guacd: &mut FakeGuacd) {
    timeout(Duration::from_secs(5), &mut guacd.selected)
        .await
        .expect("the opening reaches guacd")
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_client_that_leaves_during_the_opening_ends_the_session_operator(pool: PgPool) {
    // The opening would wait out the whole handshake if nothing noticed the client leave.
    let setup = Setup::route(|route| route.timing.handshake = Duration::from_secs(60));
    let (mut guacd, bench) = stalled(&pool, setup).await;
    let mut raw = TcpStream::connect(bench.served).await.unwrap();
    let request = format!(
        "GET /tunnel?token={} HTTP/1.1\r\nHost: {}\r\nConnection: Upgrade\r\n\
         Upgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Protocol: guacamole\r\n\r\n",
        bench.token, bench.served
    );
    raw.write_all(request.as_bytes()).await.unwrap();
    opening_reached_guacd(&mut guacd).await;
    drop(raw);
    // The client left first, so the session ends `OPERATOR` as for any client that leaves;
    // the opening stops and guacd gets no credential.
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client left before the SSH session opened."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_d_10_end_session_during_the_opening_sends_guacd_no_credential(pool: PgPool) {
    let setup = Setup::route(|route| route.timing.handshake = Duration::from_secs(60));
    let (mut guacd, bench) = stalled(&pool, setup).await;
    let url = bench.url.clone();
    let client = tokio::spawn(async move { open_with(&url, true).await.err() });
    opening_reached_guacd(&mut guacd).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let row = end_session(&pool, &bench.live, bench.session, &by).await;
    assert!(row.unwrap().end.is_some(), "the tunnel ended its row");
    // Only now does guacd offer its `args`: a hub still in the handshake would answer with
    // the credential.
    guacd.act.send(Act::Close).ok();
    let after_args = timeout(Duration::from_secs(5), guacd.handshake)
        .await
        .unwrap()
        .unwrap();
    assert!(
        after_args.iter().all(|i| i.opcode != "connect"),
        "{after_args:?}"
    );
    assert!(!format!("{after_args:?}").contains(PASSWORD));
    assert_eq!(client.await.unwrap(), Some(StatusCode::SERVICE_UNAVAILABLE));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_session_task_that_panics_still_ends_the_session(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    let redeemed = bench
        .live
        .redeem(&bench.token, address(LOOPBACK), now())
        .unwrap();
    let route = Arc::new(bench.route.clone());
    // As `relaying::run` holds the session from its first line.
    let task = tokio::spawn(async move {
        let _unfinished = Unfinished::new(route, redeemed);
        panic!("the session task fails");
    });
    assert!(task.await.unwrap_err().is_panic());
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The hub's tunnel stopped unexpectedly and closed the SSH session."
    );
}
