//! Each K4 Section 5 cause on the wire: its close code, end reason, event severity and the
//! one `session.end` audit entry.

use std::time::Duration;

use chrono::TimeDelta;
use guac_protocol::{Instruction, encode};
use sqlx::PgPool;
use tokio::time::timeout;

use super::{
    bench::{Setup, address, now, operator},
    ended_once,
    fake_guacd::Act,
    records::{ended, stored_text},
    session,
};
use crate::sessions::{Actor, EndReason, end::end_session};

const LOOPBACK: &str = "127.0.0.1";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_end_session_closes_1000_operator_audited_by_who_asked(pool: PgPool) {
    let (_guacd, bench, mut client) = session(&pool, Setup::default()).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let row = end_session(&pool, &bench.live, bench.session, &by)
        .await
        .unwrap();
    assert_eq!(row.end.map(|end| end.reason), Some(EndReason::Operator));
    assert_eq!(client.close_code().await, Some(1000));
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
async fn k4_5_guacd_disconnect_is_relayed_then_1000_remote_closed(pool: PgPool) {
    let (guacd, bench, mut client) = session(&pool, Setup::default()).await;
    guacd.act.send(Act::Send("10.disconnect;".to_owned())).ok();
    assert_eq!(client.text().await, "10.disconnect;");
    assert_eq!(client.close_code().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "remote_closed",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_guacd_closing_its_socket_is_1000_remote_closed(pool: PgPool) {
    let (guacd, bench, mut client) = session(&pool, Setup::default()).await;
    guacd.act.send(Act::Close).ok();
    assert_eq!(client.close_code().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "remote_closed",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_a_reset_guacd_connection_is_1011_gateway_error(pool: PgPool) {
    let (guacd, bench, mut client) = session(&pool, Setup::default()).await;
    guacd.act.send(Act::Reset).ok();
    assert_eq!(client.close_code().await, Some(1011));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The connection to the gateway failed, so the hub closed the session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_a_guacd_that_stops_reading_is_gateway_error(pool: PgPool) {
    let setup = Setup::route(|route| route.timing.client_idle = Duration::from_millis(500));
    let (guacd, bench, mut client) = session(&pool, setup).await;
    guacd.act.send(Act::Stall).ok();
    // Clipboard data until the hub's writes to guacd stall and the hub stops reading the
    // client: seven of the largest instructions per message.
    client.send("9.clipboard,1.0,10.text/plain;").await;
    let data = "A".repeat(8_000);
    let blobs = format!("4.blob,1.0,8000.{data};").repeat(7);
    client.flood(&blobs, 4_000).await;
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The gateway stopped reading what the hub sent, so the hub closed the session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_r_035_guacd_error_is_relayed_with_fixed_text_then_1011_gateway_error(pool: PgPool) {
    let (guacd, bench, mut client) = session(&pool, Setup::default()).await;
    guacd
        .act
        .send(Act::Send(encode(&Instruction::new(
            "error",
            ["Upstream at 10.9.9.9 died", "519"],
        ))))
        .ok();
    assert_eq!(client.text().await, "5.error,18.Aborted. See logs.,3.519;");
    assert_eq!(client.close_code().await, Some(1011));
    ended_once(
        &ended(&pool, bench.session).await,
        "gateway_error",
        "warning",
        LOOPBACK,
    );
    assert!(!stored_text(&pool).await.contains("10.9.9.9"), "D-11");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_guacd_unauthorized_or_forbidden_is_1011_auth_failed(pool: PgPool) {
    let (guacd, bench, mut client) = session(&pool, Setup::default()).await;
    guacd
        .act
        .send(Act::Send("5.error,6.denied,3.769;".to_owned()))
        .ok();
    assert_eq!(client.text().await, "5.error,18.Aborted. See logs.,3.769;");
    assert_eq!(client.close_code().await, Some(1011));
    ended_once(
        &ended(&pool, bench.session).await,
        "auth_failed",
        "warning",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_011_r_034_an_instruction_that_does_not_decode_is_1002_protocol_error(pool: PgPool) {
    let (_guacd, bench, mut client) = session(&pool, Setup::default()).await;
    client.send("3.key,5.65307;garbage").await;
    assert_eq!(client.close_code().await, Some(1002));
    ended_once(
        &ended(&pool, bench.session).await,
        "protocol_error",
        "warning",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_011_gw_013_an_opcode_outside_the_allow_list_is_1002(pool: PgPool) {
    let (mut guacd, bench, mut client) = session(&pool, Setup::default()).await;
    client.send("4.argv,1.1,10.text/plain,8.hostname;").await;
    assert_eq!(client.close_code().await, Some(1002));
    ended_once(
        &ended(&pool, bench.session).await,
        "protocol_error",
        "warning",
        LOOPBACK,
    );
    assert_eq!(
        super::heard(&mut guacd).await.opcode,
        "disconnect",
        "nothing else reached guacd"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_011_a_binary_message_is_1002_protocol_error(pool: PgPool) {
    let (_guacd, bench, mut client) = session(&pool, Setup::default()).await;
    client.send_binary(b"3.key,5.65307,1.1;").await;
    assert_eq!(client.close_code().await, Some(1002));
    ended_once(
        &ended(&pool, bench.session).await,
        "protocol_error",
        "warning",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_4_a_silent_client_is_4002_timeout(pool: PgPool) {
    let setup = Setup::route(|route| route.timing.client_idle = Duration::from_millis(900));
    let (_guacd, bench, mut client) = session(&pool, setup).await;
    assert_eq!(client.close_code().await, Some(4002));
    ended_once(
        &ended(&pool, bench.session).await,
        "timeout",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_4_pings_keep_a_client_from_timing_out(pool: PgPool) {
    // Four pings 300 ms apart span more than the idle wait; each gap is a third of it.
    let setup = Setup::route(|route| route.timing.client_idle = Duration::from_millis(900));
    let (_guacd, bench, mut client) = session(&pool, setup).await;
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_millis(300)).await;
        client.send("0.,4.ping,1.1;").await;
        assert_eq!(client.text().await, "0.,4.ping,1.1;");
    }
    assert_eq!(client.close().await, Some(1000));
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
async fn k4_5_d_10_hub_shutdown_closes_1001_and_finishes(pool: PgPool) {
    let (_guacd, bench, mut client) = session(&pool, Setup::default()).await;
    let (closing, _) = bench.live.shut_down();
    assert_eq!(client.close_code().await, Some(1001));
    timeout(Duration::from_secs(5), closing.finished())
        .await
        .expect("the tunnel deregisters");
    ended_once(
        &ended(&pool, bench.session).await,
        "hub_shutdown",
        "info",
        LOOPBACK,
    );
}
