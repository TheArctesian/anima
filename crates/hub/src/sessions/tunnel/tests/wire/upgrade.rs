//! Between the response and the upgraded socket (K4 Section 5, D-9): a client that never
//! completes the upgrade, `endSession` or shutdown before it, and no upgrade in time. guacd
//! is told to disconnect whenever it was reached; a changed certificate stays the cause.

use std::time::Duration;

use axum::extract::ws::WebSocket;
use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::sync::oneshot;

use super::{
    super::super::{
        ending::{Cause, End},
        opening::{self, Opening},
        relaying::connected,
    },
    Bench,
    bench::{LOOPBACK, Presents, Setup, address, bench, now, on_tunnel, operator},
    ended_once,
    fake_guacd::{FakeGuacd, RDP_ARGS, Script, fake_guacd},
    heard,
    records::ended,
};
use crate::sessions::{Actor, Redeemed, end::finish, upgrade::FarSide as _};

/// A session redeemed and opened to a guacd past `ready`, with no client socket yet.
async fn opened(pool: &PgPool, setup: Setup) -> (FakeGuacd, Bench, Redeemed, Opening) {
    let guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let bench = bench(pool, &guacd.address, setup).await;
    let redeemed = bench.live.redeem(on_tunnel(&bench.token), now()).unwrap();
    let Ok(opening) = opening::open(&bench.route, &redeemed.ticket).await else {
        panic!("the gateway side opens");
    };
    (guacd, bench, redeemed, opening)
}

/// Runs the tunnel after the response with `socket` as the upgrade.
async fn after_response(
    bench: &Bench,
    redeemed: &mut Redeemed,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) -> End {
    let ended = connected(bench.route.timing, &mut redeemed.tunnel, opening, socket);
    tokio::time::timeout(Duration::from_secs(5), ended)
        .await
        .expect("the tunnel ends without an upgraded socket")
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_failed_upgrade_is_operator_and_guacd_is_told(pool: PgPool) {
    let (mut guacd, bench, mut redeemed, opening) = opened(&pool, Setup::default()).await;
    let (socket_tx, socket) = oneshot::channel();
    drop(socket_tx);
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::OperatorClosed);
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_end_session_before_the_upgrade_wins_and_names_who_asked(pool: PgPool) {
    let (mut guacd, bench, mut redeemed, opening) = opened(&pool, Setup::default()).await;
    let (_socket_tx, socket) = oneshot::channel();
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    bench.live.end_by(bench.session, &by);
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!((end.cause, end.by), (Cause::OperatorClosed, Some(by)));
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_10_shutdown_before_the_upgrade_is_hub_shutdown(pool: PgPool) {
    let (mut guacd, bench, mut redeemed, opening) = opened(&pool, Setup::default()).await;
    let (_socket_tx, socket) = oneshot::channel();
    let _closing = bench.live.shut_down();
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::HubShutdown);
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_3_no_upgrade_within_the_handshake_wait_is_timeout(pool: PgPool) {
    let setup = Setup::route(|route| route.timing.handshake = Duration::from_millis(300));
    let (mut guacd, bench, mut redeemed, opening) = opened(&pool, setup).await;
    let (_socket_tx, socket) = oneshot::channel();
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::Timeout);
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
}

/// A changed certificate, then a client that leaves (`leaves`) or never upgrades: the session
/// ends `SERVER_CERTIFICATE_MISMATCH` with its Error event, not as the upgrade ended.
async fn mismatch_without_upgrade(pool: &PgPool, leaves: bool) {
    let setup = Setup {
        route: Box::new(|route| route.timing.handshake = Duration::from_millis(300)),
        ..Setup::presents(Presents::Changed)
    };
    let (mut guacd, bench, mut redeemed, opening) = opened(pool, setup).await;
    assert!(matches!(opening, Opening::CertificateChanged));
    let (socket_tx, socket) = oneshot::channel();
    if leaves {
        drop(socket_tx);
    }
    let end = after_response(&bench, &mut redeemed, opening, socket).await;
    assert_eq!(end.cause, Cause::ServerCertificateMismatch);
    finish(&bench.route.records(), redeemed, &end).await;
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "server_certificate_mismatch", "error", LOOPBACK);
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_changed_certificate_stays_the_cause_when_the_client_leaves(pool: PgPool) {
    mismatch_without_upgrade(&pool, true).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_changed_certificate_stays_the_cause_without_an_upgrade_in_time(pool: PgPool) {
    mismatch_without_upgrade(&pool, false).await;
}
