//! Each K4 Section 5 cause on the wire: its close code, end reason, event severity and the
//! one `session.end` audit entry.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::TimeDelta;
use guac_protocol::{Instruction, encode};
use sqlx::PgPool;
use tokio::time::timeout;

use super::super::super::opening::{HostKeyUnread, ReadHostKey};
use super::{
    bench::{Setup, address, ended, now, operator, stored_text},
    ended_once,
    fake_guacd::{ARGS_1_6_0, Act, Script, fake_guacd},
    session,
    socket::open,
};
use crate::sessions::{
    Actor, EndReason, PublicKey,
    end::end_session,
    tests::{ECDSA, ED25519},
};

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

/// A host-key reader that presents `key` and remembers the address it was asked about.
fn presenting(key: &'static str, asked: Arc<Mutex<Option<SocketAddr>>>) -> ReadHostKey {
    Arc::new(move |target| {
        *asked.lock().unwrap() = Some(target);
        Box::pin(async move { PublicKey::from_base64(key).map_err(|_| HostKeyUnread) })
    })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_sec_007_a_changed_host_key_closes_4001_before_guacd(pool: PgPool) {
    let mut guacd = fake_guacd(Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    })
    .await;
    let asked = Arc::default();
    let reader = presenting(ECDSA, Arc::clone(&asked));
    let setup = Setup::route(move |route| route.read_host_key = Some(reader));
    let bench = super::bench(&pool, &guacd.address, setup).await;
    let mut client = open(&bench.url).await;
    assert_eq!(client.text().await, "5.error,16.Host key changed,3.771;");
    assert_eq!(client.close_code().await, Some(4001));
    ended_once(
        &ended(&pool, bench.session).await,
        "host_key_mismatch",
        "error",
        LOOPBACK,
    );
    assert_eq!(
        *asked.lock().unwrap(),
        Some("10.10.0.21:22".parse().unwrap())
    );
    let never = timeout(Duration::from_millis(200), &mut guacd.handshake).await;
    assert!(never.is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_the_accepted_host_key_opens_the_session(pool: PgPool) {
    let reader = presenting(ED25519, Arc::default());
    let setup = Setup::route(move |route| route.read_host_key = Some(reader));
    let (_guacd, bench, mut client) = session(&pool, setup).await;
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
    let setup = Setup::route(|route| route.timing.client_idle = Duration::from_millis(300));
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
    let setup = Setup::route(|route| route.timing.client_idle = Duration::from_millis(400));
    let (_guacd, bench, mut client) = session(&pool, setup).await;
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_millis(200)).await;
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
    let closing = bench.live.shut_down();
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
