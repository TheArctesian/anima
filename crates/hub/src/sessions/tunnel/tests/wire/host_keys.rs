//! The hub's own read of the device's host key before the guacd handshake (R-034, SEC-007),
//! with the SSH probe against a real SSH server: a changed key, an unreadable key, a host
//! that does not resolve, and the checked address guacd is sent to (R-032).

use std::time::Duration;

use sqlx::PgPool;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{LOOPBACK, Setup, move_device},
    ended_once,
    fake_guacd::{ARGS_1_6_0, Script, connect_values, fake_guacd},
    records::ended,
    socket::{open, open_with},
};
use crate::sessions::{PublicKey, tests::ECDSA};

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
    let setup = Setup {
        accepted: Some(PublicKey::from_base64(ECDSA).unwrap()),
        ..Setup::default()
    };
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
    let never = timeout(Duration::from_millis(200), &mut guacd.selected).await;
    assert!(never.is_err(), "guacd is never reached");
}

/// The device moved to `host`; opening the tunnel is 503 `GATEWAY_ERROR` with `message`,
/// and guacd is never reached.
async fn refused_at(pool: &PgPool, host: &str, port: Option<u16>, message: &str) {
    let mut guacd = fake_guacd(Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    })
    .await;
    let bench = super::bench(pool, &guacd.address, Setup::default()).await;
    move_device(pool, host, port.unwrap_or(bench.ssh_port)).await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(ended.events[0].3, message);
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_034_a_host_key_that_cannot_be_read_is_503_gateway_error(pool: PgPool) {
    // Nothing listens on the discard port, so the probe cannot reach the host.
    let message = "The hub could not read the device's SSH host key.";
    refused_at(&pool, LOOPBACK, Some(9), message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_a_host_that_does_not_resolve_is_503_gateway_error(pool: PgPool) {
    let message = "The device's host name does not resolve.";
    refused_at(&pool, "no-such-device.invalid", None, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_r_034_the_probe_and_guacd_get_the_checked_address(pool: PgPool) {
    // An IPv4-mapped host is checked as the IPv4 address it is, and guacd gets that address
    // as `hostname` and in the pinned `host-key` line alike, or libssh2 would never match.
    let mut guacd = fake_guacd(Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    })
    .await;
    let bench = super::bench(&pool, &guacd.address, Setup::default()).await;
    move_device(&pool, "::ffff:127.0.0.1", bench.ssh_port).await;
    let mut client = open(&bench.url).await;
    super::opened(&mut client).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    let values = connect_values(ARGS_1_6_0, &handshake[6]);
    assert_eq!(values["hostname"], LOOPBACK);
    let key = bench.host_key.to_base64();
    let line = format!(
        "[{}]:{} ssh-ed25519 {key}",
        values["hostname"], values["port"]
    );
    assert_eq!(values["host-key"], line, "host-key names the hostname");
    assert_eq!(values["port"], bench.ssh_port.to_string());
    assert_eq!(client.close().await, Some(1000));
}
