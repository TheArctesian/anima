//! The hub's own host-key read before the guacd handshake (R-034, SEC-007), through the
//! reader seam that WP-10 T3's probe fills: a changed key, a refused or unresolved target, an
//! unreadable key, and the address guacd is sent to.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use sqlx::PgPool;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::super::super::opening::{HostKeyRead, HostKeyUnread, ReadHostKey};
use super::{
    bench::{Setup, ended},
    ended_once,
    fake_guacd::{ARGS_1_6_0, Script, connect_values, fake_guacd},
    session,
    socket::{open, open_with},
};
use crate::sessions::{
    PublicKey,
    tests::{ECDSA, ED25519},
};

const LOOPBACK: &str = "127.0.0.1";

/// The host and port a reader was asked about.
type Asked = Arc<Mutex<Option<(String, u16)>>>;

/// A host-key reader that presents `key` from `address` and remembers what it was asked.
fn presenting(key: &'static str, address: &str, asked: Asked) -> ReadHostKey {
    let address: SocketAddr = address.parse().unwrap();
    Arc::new(move |host, port| {
        *asked.lock().unwrap() = Some((host, port));
        Box::pin(async move {
            let key = PublicKey::from_base64(key).map_err(|_| HostKeyUnread::Unreadable)?;
            Ok(HostKeyRead { key, address })
        })
    })
}

/// A host-key reader that finds no key, for `unread`.
fn failing(unread: HostKeyUnread) -> ReadHostKey {
    Arc::new(move |_, _| Box::pin(async move { Err(unread) }))
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
    let asked = Asked::default();
    let reader = presenting(ECDSA, "10.10.0.21:22", Arc::clone(&asked));
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
    assert_eq!(*asked.lock().unwrap(), Some(("10.10.0.21".to_owned(), 22)));
    let never = timeout(Duration::from_millis(200), &mut guacd.handshake).await;
    assert!(never.is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_r_034_a_reader_that_refuses_the_target_is_503_before_guacd(pool: PgPool) {
    let mut guacd = fake_guacd(Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    })
    .await;
    let reader = failing(HostKeyUnread::RefusedTarget);
    let setup = Setup::route(move |route| route.read_host_key = Some(reader));
    let bench = super::bench(&pool, &guacd.address, setup).await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert!(ended.events[0].3.contains("loopback"), "{:?}", ended.events);
    assert!(
        guacd.handshake.try_recv().is_err(),
        "guacd is never reached"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_an_unreadable_host_key_is_503_gateway_error(pool: PgPool) {
    unread_is_503(
        &pool,
        HostKeyUnread::Unreadable,
        "The hub could not read the device's SSH host key.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_a_host_the_reader_cannot_resolve_is_503_gateway_error(pool: PgPool) {
    unread_is_503(
        &pool,
        HostKeyUnread::Unresolved,
        "The device's host name does not resolve.",
    )
    .await;
}

/// A reader failing with `unread` ends the session `GATEWAY_ERROR` with `message`.
async fn unread_is_503(pool: &PgPool, unread: HostKeyUnread, message: &str) {
    let reader = failing(unread);
    let setup = Setup::route(move |route| route.read_host_key = Some(reader));
    let bench = super::bench(pool, "127.0.0.1:9", setup).await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(ended.events[0].3, message);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_sec_007_the_accepted_key_opens_to_the_address_it_was_read_from(pool: PgPool) {
    // The reader resolved the host to another address than the literal: guacd gets that one,
    // as `hostname` and in the pinned `host-key` line alike, or libssh2 would never match.
    let reader = presenting(ED25519, "10.10.0.99:22", Asked::default());
    let setup = Setup::route(move |route| route.read_host_key = Some(reader));
    let (mut guacd, bench, mut client) = session(&pool, setup).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    let values = connect_values(ARGS_1_6_0, &handshake[6]);
    assert_eq!(values["hostname"], "10.10.0.99");
    let line = format!("{} ssh-ed25519 {ED25519}", values["hostname"]);
    assert_eq!(values["host-key"], line, "host-key names the hostname");
    assert_eq!(client.close().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
}
