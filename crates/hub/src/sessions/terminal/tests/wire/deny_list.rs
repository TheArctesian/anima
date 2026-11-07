//! The deny list on the SSH terminal (R-030, R-032, security): a device whose host is, or resolves
//! into, an `ANIMA_PROBE_DENY` network is 503 `GATEWAY_ERROR` before any connection to it,
//! whatever form the address takes; a host outside the list still opens.

use std::time::Duration;

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{END_WAIT, Setup, bench},
    session,
    socket::open_with,
};
use crate::{
    sessions::{
        Live,
        tests::fixture::{LOOPBACK, ended, ended_once, move_device},
    },
    targets::AddressPolicy,
};

/// The tests' loopback rule, with `cidr` denied.
fn denying(cidr: &str) -> Setup {
    let policy = AddressPolicy::new(vec![cidr.parse().unwrap()]);
    Setup {
        live: Live::with_tunnel_end_wait(END_WAIT)
            .reaching_loopback_for_tests()
            .denying(policy),
        ..Setup::default()
    }
}

/// The device moved to `host` on a listener of its own while loopback is denied: opening
/// is 503 `GATEWAY_ERROR` and the listener never sees a connection.
async fn refused_at(pool: &PgPool, host: &str) {
    let mut bench = bench(pool, denying("127.0.0.0/8")).await;
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
    assert_eq!(
        bench.ssh.heard_so_far(),
        [],
        "the fake server is never reached"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_a_host_literal_in_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, LOOPBACK).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_a_name_resolving_into_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, "localhost").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_the_ipv4_mapped_form_of_a_denied_address_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, "::ffff:127.0.0.1").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_a_host_outside_the_deny_list_still_opens(pool: PgPool) {
    let (_bench, mut client) = session(&pool, denying("192.0.2.0/24")).await;
    assert_eq!(client.close().await, Some(1000));
}
