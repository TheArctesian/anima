//! Between the response and the upgraded socket (K4 Sections 8.1 and 8.6, D-9): a failed
//! upgrade, `endSession` before it, no upgrade in time, and a changed host key whose upgrade
//! fails. The device's channel is closed whenever the hub signed in.

use crate::sessions::Presented;
use std::time::Duration;

use axum::extract::ws::WebSocket;
use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::sync::oneshot;

use super::{
    super::super::{
        opening::{self, Opening},
        session::connected,
    },
    Bench, WAIT,
    bench::{Setup, bench},
};
use crate::sessions::{
    Actor, Redeemed,
    end::{Cause, End, finish},
    tests::fixture::{LOOPBACK, address, ended, ended_once, now, operator},
    upgrade::FarSide as _,
};

/// A session redeemed and signed in, with no client socket yet.
async fn signed_in(pool: &PgPool, setup: Setup) -> (Bench, Redeemed, Opening) {
    let bench = bench(pool, setup).await;
    let redeemed = bench
        .live
        .redeem(Presented::terminal(&bench.token, address(LOOPBACK)), now())
        .unwrap();
    let Ok(opening @ Opening::SignedIn(_)) = opening::open(&bench.terminal, &redeemed.ticket).await
    else {
        panic!("the device side opens");
    };
    (bench, redeemed, opening)
}

/// Runs the session after the response with `socket` as the upgrade.
async fn after_response(
    bench: &Bench,
    redeemed: &mut Redeemed,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) -> End {
    let timing = bench.terminal.timing;
    let ended = connected(timing, &mut redeemed.tunnel, opening, socket);
    tokio::time::timeout(WAIT, ended)
        .await
        .expect("the session ends without an upgraded socket")
}

/// The hub closed the one channel it opened.
async fn channel_closed(bench: &Bench) {
    tokio::time::timeout(WAIT, async {
        while bench.ssh.ledger.lock().unwrap().closed.is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the hub closes the session channel");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_failed_upgrade_is_operator_and_the_channel_is_closed(pool: PgPool) {
    let (bench, mut redeemed, opening) = signed_in(&pool, Setup::default()).await;
    let (socket_tx, socket) = oneshot::channel();
    drop(socket_tx);
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::OperatorClosed);
    channel_closed(&bench).await;
    finish(&bench.terminal.records(), redeemed, &end).await;
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_end_session_before_the_upgrade_wins_and_names_who_asked(pool: PgPool) {
    let (bench, mut redeemed, opening) = signed_in(&pool, Setup::default()).await;
    let (_socket_tx, socket) = oneshot::channel();
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    bench.live.end_by(bench.session, &by);
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!((end.cause, end.by), (Cause::OperatorClosed, Some(by)));
    channel_closed(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_no_upgrade_within_the_upgrade_wait_is_timeout(pool: PgPool) {
    let setup = Setup::terminal(|terminal| terminal.timing.upgrade = Duration::from_millis(300));
    let (bench, mut redeemed, opening) = signed_in(&pool, setup).await;
    let (_socket_tx, socket) = oneshot::channel();
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::Timeout);
    channel_closed(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_changed_host_key_stays_the_cause_when_the_upgrade_fails(pool: PgPool) {
    let bench = bench(&pool, Setup::default()).await;
    let mut redeemed = bench
        .live
        .redeem(Presented::terminal(&bench.token, address(LOOPBACK)), now())
        .unwrap();
    let (socket_tx, socket) = oneshot::channel();
    drop(socket_tx);
    let end = after_response(&bench, &mut redeemed, Opening::HostKeyChanged, socket).await;
    assert_eq!(end.cause, Cause::HostKeyMismatch);
    finish(&bench.terminal.records(), redeemed, &end).await;
    ended_once(
        &ended(&pool, bench.session).await,
        "host_key_mismatch",
        "error",
        LOOPBACK,
    );
}
