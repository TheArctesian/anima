//! Interrupted sessions end once (MV-003): one row end, one `session.end` entry, one event,
//! whichever side goes away and however two causes race.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{Setup, address, bench, now, operator},
    ended_once,
    fake_guacd::{Act, Script, fake_guacd},
    heard,
    records::ended,
    session,
    socket::open_with,
};
use crate::sessions::{Actor, EndReason, SessionEnd, end::end_session, store};

const LOOPBACK: &str = "127.0.0.1";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_client_that_drops_mid_stream_ends_the_session_once(pool: PgPool) {
    let (mut guacd, bench, client) = session(&pool, Setup::default()).await;
    client.drop_connection();
    assert_eq!(
        heard(&mut guacd).await.opcode,
        "disconnect",
        "guacd is told"
    );
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
async fn mv_003_guacd_dropping_mid_handshake_is_503_and_ends_once(pool: PgPool) {
    let guacd = fake_guacd(Script::DropAfterSelect).await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The gateway did not open the SSH session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_end_session_racing_a_client_close_ends_once(pool: PgPool) {
    let (_guacd, bench, mut client) = session(&pool, Setup::default()).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let (closed, row) = tokio::join!(
        client.close(),
        end_session(&pool, &bench.live, bench.session, &by)
    );
    assert!(closed.is_some());
    assert!(
        row.unwrap().end.is_some(),
        "endSession returns the ended row"
    );
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "operator");
    assert_eq!((ended.audits.len(), ended.events.len()), (1, 1));
    // Nothing more is written later.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let again = super::records::ended(&pool, bench.session).await;
    assert_eq!((again.audits.len(), again.events.len()), (1, 1));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_a_row_already_ended_elsewhere_gets_no_second_end_or_event(pool: PgPool) {
    let (mut guacd, bench, mut client) = session(&pool, Setup::default()).await;
    // As `close_stale` would leave it after a hub restart.
    let first = SessionEnd {
        at: now(),
        reason: EndReason::HubShutdown,
    };
    assert!(store::end(&pool, bench.session, first).await.unwrap());
    assert_eq!(client.close().await, Some(1000));
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
    // The tunnel's end write comes after it leaves guacd; give it a moment.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "hub_shutdown", "the first end wins");
    assert!(
        ended.audits.is_empty() && ended.events.is_empty(),
        "{ended:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_end_session_wins_over_a_client_that_stopped_reading(pool: PgPool) {
    // The client reads nothing more while guacd sends far more than the sockets hold, so the
    // hub's write to the client stalls; the idle wait is long, so only the end signal can
    // end it in time.
    let (guacd, bench, _client) = session(&pool, Setup::default()).await;
    let blob = format!("4.blob,1.0,8000.{};", "A".repeat(8_000));
    guacd.act.send(Act::Send(blob.repeat(4_000))).ok();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let started = Instant::now();
    let row = end_session(&pool, &bench.live, bench.session, &by)
        .await
        .unwrap();
    assert_eq!(row.end.map(|end| end.reason), Some(EndReason::Operator));
    let took = started.elapsed();
    assert!(took < Duration::from_secs(4), "{took:?}");
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
}
