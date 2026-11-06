//! The session ends once when it is interrupted during the opening (K4 Section 8.2, MV-003,
//! SEC-002, D-10): `endSession` while the device stalls in the key exchange or the sign-in,
//! hub shutdown, and the client leaving. Each ends the session `OPERATOR` or `HUB_SHUTDOWN`
//! with one row end, audit entry and event, and the hub goes no further on the device.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::oneshot, time::timeout};

use super::{
    Bench, WAIT,
    bench::{Setup, bench},
    fake_ssh::{Heard, Hold, Script},
    socket::{request, status_line},
};
use crate::sessions::{
    Actor,
    end::end_session,
    tests::fixture::{LOOPBACK, address, ended, ended_once, move_device, now, operator},
};

/// The sign-in held by the server, and an opening that would outlast every test wait.
fn held_sign_in() -> Setup {
    Setup {
        script: Script {
            hold: Hold::SignIn,
            ..Script::default()
        },
        ..Setup::terminal(|terminal| terminal.timing.opening = Duration::from_secs(60))
    }
}

fn by() -> Actor {
    Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    }
}

/// `endSession` ends the session in time, the request is answered 503, and the session ends
/// `OPERATOR` for who asked.
async fn ended_by_end_session(pool: &PgPool, bench: &Bench, mut raw: tokio::net::TcpStream) {
    let row = timeout(WAIT, end_session(pool, &bench.live, bench.session, &by()))
        .await
        .expect("endSession returns in time")
        .unwrap();
    assert!(row.end.is_some(), "the session ended its row in time");
    assert_eq!(
        status_line(&mut raw).await,
        "HTTP/1.1 503 Service Unavailable"
    );
    ended_once(
        &ended(pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_d_10_end_session_during_the_sign_in_is_operator_and_opens_no_channel(
    pool: PgPool,
) {
    let mut bench = bench(&pool, held_sign_in()).await;
    let raw = request(bench.served, &bench.token).await;
    assert_eq!(bench.ssh.next().await, Heard::Auth("password"));
    ended_by_end_session(&pool, &bench, raw).await;
    bench.ssh.release();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        bench.ssh.ledger.lock().unwrap().opened.is_empty(),
        "no channel after the end"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_d_10_end_session_during_the_key_exchange_is_operator(pool: PgPool) {
    let setup = Setup::terminal(|terminal| terminal.timing.opening = Duration::from_secs(60));
    let mut bench = bench(&pool, setup).await;
    // A device that takes the connection and never says a word.
    let silent = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    move_device(&pool, LOOPBACK, silent.local_addr().unwrap().port()).await;
    let (accepted_tx, accepted) = oneshot::channel();
    tokio::spawn(async move {
        let connection = silent.accept().await.unwrap();
        let _ = accepted_tx.send(());
        std::future::pending::<()>().await;
        drop(connection);
    });
    let raw = request(bench.served, &bench.token).await;
    timeout(WAIT, accepted).await.unwrap().unwrap();
    ended_by_end_session(&pool, &bench, raw).await;
    assert_eq!(
        bench.ssh.heard_so_far(),
        [],
        "the fake server is not reached"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_10_shutdown_during_the_opening_is_hub_shutdown(pool: PgPool) {
    let mut bench = bench(&pool, held_sign_in()).await;
    let mut raw = request(bench.served, &bench.token).await;
    assert_eq!(bench.ssh.next().await, Heard::Auth("password"));
    let (closing, _) = bench.live.shut_down();
    timeout(WAIT, closing.finished())
        .await
        .expect("the session deregisters");
    assert_eq!(
        status_line(&mut raw).await,
        "HTTP/1.1 503 Service Unavailable"
    );
    ended_once(
        &ended(&pool, bench.session).await,
        "hub_shutdown",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_client_that_leaves_during_the_opening_is_operator(pool: PgPool) {
    let mut bench = bench(&pool, held_sign_in()).await;
    let raw = request(bench.served, &bench.token).await;
    assert_eq!(bench.ssh.next().await, Heard::Auth("password"));
    drop(raw);
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client left before the SSH session opened."
    );
    bench.ssh.release();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        bench.ssh.ledger.lock().unwrap().opened.is_empty(),
        "no channel after the end"
    );
}
