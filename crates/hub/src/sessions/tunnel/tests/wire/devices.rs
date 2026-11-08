//! The device as the opening finds it (K4 Sections 2 and 3, R-032): each protocol turned
//! off after the start, the accepted certificate gone, a host that does not resolve, and the
//! checked address, values, drive and audio guacd gets for RDP and VNC.

use std::collections::HashMap;

use sqlx::PgPool;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{DRIVE_ROOT, LOOPBACK, PASSWORD, Setup, VNC_PASSWORD, docker_host, move_host},
    ended_once,
    fake_guacd::{FakeGuacd, RDP_ARGS, Script, VNC_ARGS, connect_values, fake_guacd},
    opened,
    records::ended,
    socket::{open, open_with},
};
use crate::sessions::Protocol;

async fn serving(args: &'static [u8]) -> FakeGuacd {
    fake_guacd(Script::Serve {
        args,
        with_ready: "",
    })
    .await
}

/// After `change` to the started session's device, opening the tunnel is 503
/// `GATEWAY_ERROR` with `message`, and guacd is never reached.
async fn refused_after(pool: &PgPool, protocol: Protocol, change: &str, message: &str) {
    let mut guacd = serving(RDP_ARGS).await;
    let bench = super::bench(pool, &guacd.address, Setup::protocol(protocol)).await;
    sqlx::query(change)
        .bind(docker_host())
        .execute(pool)
        .await
        .unwrap();
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
async fn k4_2_rdp_turned_off_after_start_is_503_gateway_error(pool: PgPool) {
    let off = "update devices.rdp_settings set enabled = false where device_id = $1";
    let message = "RDP was turned off for the device before the session opened.";
    refused_after(&pool, Protocol::Rdp, off, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_2_vnc_turned_off_after_start_is_503_gateway_error(pool: PgPool) {
    let off = "update devices.vnc_settings set enabled = false where device_id = $1";
    let message = "VNC was turned off for the device before the session opened.";
    refused_after(&pool, Protocol::Vnc, off, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_an_rdp_session_without_an_accepted_certificate_is_503(pool: PgPool) {
    // A changed device record: deleting the device would cascade the certificate too.
    let gone = "delete from trust.server_certificates where device_id = $1";
    let message = "The device has no accepted server certificate. Test the connection and \
                   accept its certificate.";
    refused_after(&pool, Protocol::Rdp, gone, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_a_host_that_does_not_resolve_is_503_gateway_error(pool: PgPool) {
    let mut guacd = serving(RDP_ARGS).await;
    let bench = super::bench(&pool, &guacd.address, Setup::default()).await;
    move_host(&pool, "no-such-device.invalid").await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The device's host name does not resolve."
    );
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}

/// Starts the bench's session, applies `changes` (each bound to `docker-host`), opens it,
/// then returns what guacd was sent.
async fn handshake_of(
    pool: &PgPool,
    setup: Setup,
    args: &'static [u8],
    changes: &[&str],
) -> Handshake {
    let mut guacd = serving(args).await;
    let bench = super::bench(pool, &guacd.address, setup).await;
    for change in changes {
        sqlx::query(change)
            .bind(docker_host())
            .execute(pool)
            .await
            .unwrap();
    }
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let selected = (&mut guacd.selected).await.unwrap();
    let handshake = (&mut guacd.handshake).await.unwrap();
    assert_eq!(client.close().await, Some(1000));
    assert_eq!(handshake[1].opcode, "audio");
    Handshake {
        selected,
        audio: handshake[1].args.clone(),
        values: connect_values(args, &handshake[6]),
        rdp_port: bench.rdp.port.to_string(),
        accepted: bench.accepted.clone(),
    }
}

/// What guacd was sent: the protocol it was asked for, the `audio` instruction's mimetypes
/// and the `connect` values by name; with the bench's RDP port and accepted fingerprint.
struct Handshake {
    selected: String,
    audio: Vec<String>,
    values: HashMap<String, String>,
    rdp_port: String,
    accepted: String,
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_3_an_rdp_session_selects_rdp_with_the_stored_settings(pool: PgPool) {
    let Handshake {
        selected,
        audio,
        values,
        rdp_port,
        accepted,
    } = handshake_of(&pool, Setup::default(), RDP_ARGS, &[]).await;
    assert_eq!(selected, "rdp");
    assert_eq!(
        audio,
        ["audio/L8", "audio/L16"],
        "audio out is on by default"
    );
    assert_eq!(values["hostname"], LOOPBACK);
    assert_eq!(values["port"], rdp_port);
    assert_eq!(values["username"], "ops");
    assert_eq!(values["domain"], "LAB");
    assert_eq!(values["password"], PASSWORD);
    assert_eq!(values["security"], "nla");
    assert_eq!(values["ignore-cert"], "");
    assert_eq!(values["cert-fingerprints"], format!("sha256:{accepted}"));
    assert_eq!(values["drive-path"], "", "no shared drive by default");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_021_the_shared_drive_is_the_device_s_folder_under_the_configured_root(pool: PgPool) {
    let drive = "update devices.rdp_settings set shared_drive = true, audio_out = false \
                 where device_id = $1";
    let Handshake { audio, values, .. } =
        handshake_of(&pool, Setup::default(), RDP_ARGS, &[drive]).await;
    assert_eq!(audio, Vec::<String>::new(), "no sound, no audio mimetypes");
    assert_eq!(values["disable-audio"], "true");
    assert_eq!(values["enable-drive"], "true");
    assert_eq!(
        values["drive-path"],
        format!("{DRIVE_ROOT}/{}", docker_host())
    );
    assert_eq!(values["create-drive-path"], "true");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_3_a_vnc_session_selects_vnc_with_its_own_password_and_no_audio(pool: PgPool) {
    let setup = Setup::protocol(Protocol::Vnc);
    let Handshake {
        selected,
        audio,
        values,
        ..
    } = handshake_of(&pool, setup, VNC_ARGS, &[]).await;
    assert_eq!(selected, "vnc");
    assert_eq!(audio, Vec::<String>::new());
    assert_eq!(values["hostname"], LOOPBACK);
    assert_eq!(values["port"], "5900");
    assert_eq!(values["username"], "viewer");
    assert_eq!(values["password"], VNC_PASSWORD);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_an_ipv4_mapped_host_reaches_guacd_as_the_ipv4_address_it_is(pool: PgPool) {
    let mut guacd = serving(RDP_ARGS).await;
    let bench = super::bench(&pool, &guacd.address, Setup::default()).await;
    move_host(&pool, "::ffff:127.0.0.1").await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let handshake = (&mut guacd.handshake).await.unwrap();
    assert_eq!(
        connect_values(RDP_ARGS, &handshake[6])["hostname"],
        LOOPBACK
    );
    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_standard_rdp_security_opens_without_a_certificate(pool: PgPool) {
    // No TLS, so no certificate to pin: the tunnel opens and guacd gets no fingerprint.
    let changes = [
        "update devices.rdp_settings set security = 'rdp' where device_id = $1",
        "delete from trust.server_certificates where device_id = $1",
    ];
    let Handshake { values, .. } = handshake_of(&pool, Setup::default(), RDP_ARGS, &changes).await;
    assert_eq!(values["security"], "rdp");
    assert_eq!(values["cert-fingerprints"], "");
}
