//! The tunnel after the device's host changed between the start and the opening (SEC-011):
//! a password entered for another host ends the session `GATEWAY_ERROR` with one Warning event
//! that says to enter it again, and guacd never hears from the hub, so the password goes
//! nowhere. RDP is covered in the modes that pin no certificate, where nothing else would stop
//! it. The host text decides, not what it resolves to: `localhost` and `127.0.0.1` reach the
//! same address and are still two hosts.

use sqlx::PgPool;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{
        LOOPBACK, PASSWORD, Setup, VNC_PASSWORD, address, docker_host, enter_again, move_host, now,
        operator, request,
    },
    ended_once,
    fake_guacd::{FakeGuacd, RDP_ARGS, Script, VNC_ARGS, fake_guacd},
    opened,
    records::ended,
    socket::{open, open_with},
};
use crate::{
    clock::{Clock, FixedClock},
    fleet::{
        credentials::PasswordKind,
        sessions::{Protocol, start},
    },
};

async fn serving(args: &'static [u8]) -> FakeGuacd {
    fake_guacd(Script::Serve {
        args,
        with_ready: "",
    })
    .await
}

/// The session's tunnel after `changes` to `docker-host` and its move to `host`: 503
/// `GATEWAY_ERROR`, one Warning event that names neither host nor password, and guacd never
/// reached.
async fn refused_at(pool: &PgPool, protocol: Protocol, changes: &[&str], host: &str) {
    let mut guacd = serving(RDP_ARGS).await;
    let bench = super::bench(pool, &guacd.address, Setup::protocol(protocol)).await;
    for change in changes {
        sqlx::query(change)
            .bind(docker_host())
            .execute(pool)
            .await
            .unwrap();
    }
    move_host(pool, host).await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE), "{host}");
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    let message = &ended.events[0].3;
    assert!(message.contains("address changed"), "{message}");
    assert!(message.contains("Enter the password again"), "{message}");
    for withheld in [host, LOOPBACK, PASSWORD, VNC_PASSWORD] {
        assert!(!message.contains(withheld), "{message}");
    }
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_011_a_moved_vnc_password_reaches_no_gateway(pool: PgPool) {
    refused_at(&pool, Protocol::Vnc, &[], "localhost").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_011_a_moved_rdp_password_reaches_no_gateway_with_standard_security(pool: PgPool) {
    let standard = "update devices.rdp_settings set security = 'rdp' where device_id = $1";
    refused_at(&pool, Protocol::Rdp, &[standard], "localhost").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_011_a_moved_rdp_password_reaches_no_gateway_with_the_certificate_ignored(
    pool: PgPool,
) {
    let ignored = "update devices.rdp_settings set ignore_certificate = true where device_id = $1";
    refused_at(&pool, Protocol::Rdp, &[ignored], "localhost").await;
}

/// The password entered again for the new host text opens the session there, and an
/// unchanged host opens whatever its letter case.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_011_a_password_entered_again_opens_at_the_new_host(pool: PgPool) {
    let mut guacd = serving(VNC_ARGS).await;
    let bench = super::bench(&pool, &guacd.address, Setup::protocol(Protocol::Vnc)).await;
    move_host(&pool, "localhost").await;
    enter_again(&pool, &bench, PasswordKind::Vnc, "LocalHost").await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    let values = super::fake_guacd::connect_values(VNC_ARGS, &handshake[6]);
    assert_eq!(values["password"], VNC_PASSWORD);
    assert_eq!(client.close().await, Some(1000));
}

/// A refused password is kept: once the host is changed back, the next session opens with it.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_011_changing_the_host_back_opens_without_a_new_entry(pool: PgPool) {
    let mut guacd = serving(VNC_ARGS).await;
    let bench = super::bench(&pool, &guacd.address, Setup::protocol(Protocol::Vnc)).await;
    move_host(&pool, "localhost").await;
    let refused = open_with(&bench.url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    ended(&pool, bench.session).await;

    move_host(&pool, LOOPBACK).await;
    let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(FixedClock(now()));
    let ticket = start(
        &pool,
        &clock,
        &bench.live,
        &operator(),
        address(LOOPBACK),
        request(Protocol::Vnc),
    )
    .await
    .unwrap();
    let url = format!("ws://{}/tunnel?token={}", bench.served, ticket.token());
    let mut client = open(&url).await;
    opened(&mut client).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    let values = super::fake_guacd::connect_values(VNC_ARGS, &handshake[6]);
    assert_eq!(values["password"], VNC_PASSWORD);
    assert_eq!(client.close().await, Some(1000));
}
