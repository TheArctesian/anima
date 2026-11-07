//! The deny list on the Guacamole tunnel (security): an RDP or VNC session whose device host
//! is, or resolves into, an `ANIMA_PROBE_DENY` network is 503 `GATEWAY_ERROR` before any
//! connection to the device or to guacd, whatever form the address takes; a host outside the
//! list still opens.

use std::time::Duration;

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{LOOPBACK, Setup, docker_host, move_host},
    ended_once,
    fake_guacd::{RDP_ARGS, Script, VNC_ARGS, fake_guacd},
    opened,
    records::ended,
    session,
    socket::{open, open_with},
};
use crate::{
    sessions::{Live, Protocol},
    targets::AddressPolicy,
};

/// The tests' loopback rule, with `cidr` denied, for a `protocol` session.
fn denying(cidr: &str, protocol: Protocol) -> Setup {
    let policy = AddressPolicy::new(vec![cidr.parse().unwrap()]);
    Setup {
        live: Live::default()
            .reaching_loopback_for_tests()
            .denying(policy),
        ..Setup::protocol(protocol)
    }
}

/// The guacd 1.6.0 arguments for `protocol`.
fn args(protocol: Protocol) -> &'static [u8] {
    if protocol == Protocol::Vnc {
        VNC_ARGS
    } else {
        RDP_ARGS
    }
}

/// Points the started session's `protocol` port of `docker-host` at `port`.
async fn move_port(pool: &PgPool, protocol: Protocol, port: u16) {
    let change = if protocol == Protocol::Vnc {
        "update devices.vnc_settings set port = $2 where device_id = $1"
    } else {
        "update devices.rdp_settings set port = $2 where device_id = $1"
    };
    sqlx::query(change)
        .bind(docker_host())
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}

/// The device moved to `host` on a listener of its own while loopback is denied: opening
/// is 503 `GATEWAY_ERROR`, the listener never sees a connection and guacd is never reached.
async fn refused_at(pool: &PgPool, protocol: Protocol, host: &str) {
    let mut guacd = fake_guacd(Script::Serve {
        args: args(protocol),
        with_ready: "",
    })
    .await;
    let setup = denying("127.0.0.0/8", protocol);
    let bench = super::bench(pool, &guacd.address, setup).await;
    let device = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    move_host(pool, host).await;
    move_port(pool, protocol, device.local_addr().unwrap().port()).await;
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
async fn a_host_literal_in_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, Protocol::Rdp, LOOPBACK).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn a_vnc_host_literal_in_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, Protocol::Vnc, LOOPBACK).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn a_name_resolving_into_the_deny_list_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, Protocol::Rdp, "localhost").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn the_ipv4_mapped_form_of_a_denied_address_is_503_and_never_reached(pool: PgPool) {
    refused_at(&pool, Protocol::Vnc, "::ffff:127.0.0.1").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn an_rdp_host_outside_the_deny_list_still_opens(pool: PgPool) {
    let (_guacd, _bench, mut client) = session(&pool, denying("192.0.2.0/24", Protocol::Rdp)).await;
    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn a_vnc_host_outside_the_deny_list_still_opens(pool: PgPool) {
    let guacd = fake_guacd(Script::Serve {
        args: VNC_ARGS,
        with_ready: "",
    })
    .await;
    let setup = denying("192.0.2.0/24", Protocol::Vnc);
    let bench = super::bench(&pool, &guacd.address, setup).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    assert_eq!(client.close().await, Some(1000));
}
