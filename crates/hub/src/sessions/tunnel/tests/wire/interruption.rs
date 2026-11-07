//! Interrupted sessions end once (MV-003): one row end, one `session.end` entry, one event,
//! whichever side goes away and however two causes race.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{END_WAIT, Setup, address, bench, ending_within_end_wait, now, operator, patient},
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
        "The gateway did not open the RDP session."
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

/// `endSession` returned within its end wait, which is far short of the idle waits: the end
/// signal, not a timeout, ended the stalled session (D-9). The budget is the injected end
/// wait, not a fixed figure: the hub's close to the stalled side alone waits out
/// `CLOSE_WAIT`, and the end write follows, so a fixed bound near that raced a loaded
/// machine.
fn within_end_wait(took: Duration) {
    let idle = patient();
    assert!(END_WAIT < idle.client_idle.min(idle.gateway_silence));
    assert!(took < END_WAIT, "{took:?}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_end_session_wins_over_a_client_that_stopped_reading(pool: PgPool) {
    // The client reads nothing more while guacd sends far more than the sockets hold, so the
    // hub's write to the client stalls; the idle wait is long, so only the end signal can
    // end it within the end wait.
    let (guacd, bench, _client) = session(&pool, ending_within_end_wait()).await;
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
    within_end_wait(started.elapsed());
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
async fn d_9_end_session_wins_over_a_guacd_that_stopped_reading(pool: PgPool) {
    // guacd reads nothing more while the client sends clipboard data, so the hub's write to
    // guacd stalls; the idle wait is long, so only the end signal can end it within the end
    // wait.
    let (guacd, bench, mut client) = session(&pool, ending_within_end_wait()).await;
    guacd.act.send(Act::Stall).ok();
    client.send("9.clipboard,1.0,10.text/plain;").await;
    let blobs = format!("4.blob,1.0,8000.{};", "A".repeat(8_000)).repeat(7);
    client.flood(&blobs, 4_000).await;
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
    within_end_wait(started.elapsed());
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
async fn k4_5_a_client_write_stalled_past_the_idle_wait_is_4002_timeout(pool: PgPool) {
    let idle = Duration::from_secs(1);
    let setup = Setup::route(move |route| route.timing.client_idle = idle);
    let (guacd, bench, mut client) = session(&pool, setup).await;
    let blob = format!("4.blob,1.0,8000.{};", "A".repeat(8_000));
    guacd.act.send(Act::Send(blob.repeat(4_000))).ok();
    // The client reads nothing until the hub's write has stalled past the idle wait, then
    // drains while the hub's close is still on its way.
    tokio::time::sleep(idle + Duration::from_millis(500)).await;
    assert_eq!(client.close_code().await, Some(4002));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "timeout", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client stopped reading what the hub sent, so the hub closed the RDP session."
    );
}
