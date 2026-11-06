//! The deny list on the Guacamole tunnel (P2-004, security): a device whose host is, or
//! resolves into, an `ANIMA_PROBE_DENY` network is 503 `GATEWAY_ERROR` before any connection
//! to the device or to guacd, whatever form the address takes; a host outside the list still
//! opens.

use std::time::Duration;

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{LOOPBACK, Setup, move_device},
    ended_once,
    fake_guacd::{ARGS_1_6_0, Script, fake_guacd},
    records::ended,
    session,
    socket::open_with,
};
use crate::{sessions::Live, targets::AddressPolicy};

/// The tests' loopback rule, with `cidr` denied.
fn denying(cidr: &str) -> Setup {
    let policy = AddressPolicy::new(vec![cidr.parse().unwrap()]);
    Setup {
        live: Live::default()
            .reaching_loopback_for_tests()
            .denying(policy),
        ..Setup::default()
    }
}

/// The device moved to `host` on a listener of its own while loopback is denied: opening
/// is 503 `GATEWAY_ERROR`, the listener never sees a connection and guacd is never reached.
async fn refused_at(pool: &PgPool, host: &str) {
    let mut guacd = fake_guacd(Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    })
    .await;
    let bench = super::bench(pool, &guacd.address, denying("127.0.0.0/8")).await;
    let device = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    move_device(pool, host, device.local_addr().unwrap().port()).await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE), "{host}");
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    let message = &ended.events[0].3;
    assert!(message.contains("ANIMA_PROBE_DENY"), "{message}");
    assert!(
        !message.contains("127."),
        "no address in the message: {message}"
    );
    let reached = timeout(Duration::from_millis(50), device.accept()).await;
    assert!(reached.is_err(), "{host}: no connection reached the device");
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_host_literal_in_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, LOOPBACK).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_name_resolving_into_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, "localhost").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_the_ipv4_mapped_form_of_a_denied_address_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, "::ffff:127.0.0.1").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_host_outside_the_deny_list_still_opens(pool: PgPool) {
    let (_guacd, _bench, mut client) = session(&pool, denying("192.0.2.0/24")).await;
    assert_eq!(client.close().await, Some(1000));
}
