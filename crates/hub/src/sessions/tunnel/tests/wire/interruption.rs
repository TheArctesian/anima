//! Interrupted sessions end once (MV-003): one row end, one `session.end` entry, one event,
//! whichever side goes away and however two causes race.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{Setup, address, bench, ended, now, operator},
    ended_once,
    fake_guacd::{Script, fake_guacd},
    heard, session,
    socket::open_with,
};
use crate::sessions::{Actor, end::end_session};

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
    let again = super::bench::ended(&pool, bench.session).await;
    assert_eq!((again.audits.len(), again.events.len()), (1, 1));
}
