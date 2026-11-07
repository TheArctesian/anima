//! The RDP server certificate check before the guacd handshake (SEC-007, K4 Sections 3 and
//! 5): a changed certificate is `error` "Certificate changed" 0x0303, close 4003 and
//! `SERVER_CERTIFICATE_MISMATCH` with guacd never reached; the accepted one opens; standard
//! RDP security, ignored certificates and VNC read nothing; `ANY` reaches guacd as the TLS
//! security the server chose; a read that fails is 503 `GATEWAY_ERROR`.

use std::{collections::HashMap, time::Duration};

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{LOOPBACK, Presents, Setup, bench, docker_host},
    decoded, ended_once,
    fake_guacd::{RDP_ARGS, Script, VNC_ARGS, connect_values, fake_guacd},
    opened,
    records::ended,
    socket::{open, open_with},
};
use crate::sessions::Protocol;

/// The event message of a changed certificate: fixed, with no fingerprint and no address.
const MISMATCH: &str = "The device presented an RDP server certificate that is not the accepted \
                        one, so the hub closed the session before signing in. Test the \
                        connection and check the new certificate before accepting it.";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_changed_certificate_closes_4003_and_guacd_is_never_reached(pool: PgPool) {
    let guacd = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let address = guacd.local_addr().unwrap().to_string();
    let mut bench = bench(&pool, &address, Setup::presents(Presents::Changed)).await;
    let mut client = open(&bench.url).await;
    let error = decoded(&client.text().await);
    assert_eq!(error.opcode, "error");
    assert_eq!(error.args, ["Certificate changed", "771"], "0x0303");
    assert_eq!(client.close_code().await, Some(4003));

    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "server_certificate_mismatch", "error", LOOPBACK);
    let message = &ended.events[0].3;
    assert_eq!(message, MISMATCH);
    for secret in [bench.accepted.as_str(), &bench.rdp.fingerprint, LOOPBACK] {
        assert!(!message.contains(secret), "{message}");
    }
    assert!(
        bench.rdp.was_visited().await,
        "the hub read the certificate"
    );
    let accepted = timeout(Duration::from_millis(300), guacd.accept()).await;
    assert!(accepted.is_err(), "guacd sees no connection");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_the_password_is_not_loaded_before_the_certificate_check(pool: PgPool) {
    // A stored password that does not open fails the session as GATEWAY_ERROR once loaded,
    // so a mismatch here shows the check came first.
    let guacd = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let address = guacd.local_addr().unwrap().to_string();
    let bench = bench(&pool, &address, Setup::presents(Presents::Changed)).await;
    sqlx::query(
        "update credentials.secrets \
         set ciphertext = set_byte(ciphertext, 0, get_byte(ciphertext, 0) # 1) \
         where device_id = $1",
    )
    .bind(docker_host())
    .execute(&pool)
    .await
    .unwrap();
    let mut client = open(&bench.url).await;
    assert_eq!(decoded(&client.text().await).opcode, "error");
    assert_eq!(client.close_code().await, Some(4003));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "server_certificate_mismatch", "error", LOOPBACK);
}

/// Opens the session of `setup` after `changes` to `docker-host` with a guacd serving
/// `args`, closes it, and gives whether the device's RDP server saw a connection and the
/// `connect` values guacd got.
async fn read_certificate(
    pool: &PgPool,
    setup: Setup,
    args: &'static [u8],
    changes: &[&str],
) -> (bool, HashMap<String, String>) {
    let mut guacd = fake_guacd(Script::Serve {
        args,
        with_ready: "",
    })
    .await;
    let mut bench = bench(pool, &guacd.address, setup).await;
    for change in changes {
        sqlx::query(change)
            .bind(docker_host())
            .execute(pool)
            .await
            .unwrap();
    }
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let handshake = (&mut guacd.handshake).await.expect("guacd is reached");
    assert_eq!(client.close().await, Some(1000));
    let values = connect_values(args, &handshake[6]);
    (bench.rdp.was_visited().await, values)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_the_accepted_certificate_opens_the_session(pool: PgPool) {
    let (read, values) = read_certificate(&pool, Setup::default(), RDP_ARGS, &[]).await;
    assert!(read);
    assert_eq!(values["security"], "nla", "the setting is kept");
}

const ANY: &str = "update devices.rdp_settings set security = 'any' where device_id = $1";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_any_with_an_nla_server_reaches_guacd_as_nla(pool: PgPool) {
    let (read, values) = read_certificate(&pool, Setup::default(), RDP_ARGS, &[ANY]).await;
    assert!(read);
    assert_eq!(
        values["security"], "nla",
        "no fallback to standard security"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_any_with_a_tls_only_server_reaches_guacd_as_tls(pool: PgPool) {
    let setup = Setup::presents(Presents::AcceptedOverTls);
    let (read, values) = read_certificate(&pool, setup, RDP_ARGS, &[ANY]).await;
    assert!(read);
    assert_eq!(
        values["security"], "tls",
        "no fallback to standard security"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_standard_rdp_security_reads_no_certificate(pool: PgPool) {
    let standard = "update devices.rdp_settings set security = 'rdp' where device_id = $1";
    assert!(
        !read_certificate(&pool, Setup::default(), RDP_ARGS, &[standard])
            .await
            .0
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_ignored_certificates_are_not_read(pool: PgPool) {
    let ignore = "update devices.rdp_settings set ignore_certificate = true where device_id = $1";
    assert!(
        !read_certificate(&pool, Setup::default(), RDP_ARGS, &[ignore])
            .await
            .0
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_vnc_session_reads_no_certificate(pool: PgPool) {
    let setup = Setup::protocol(Protocol::Vnc);
    assert!(!read_certificate(&pool, setup, VNC_ARGS, &[]).await.0);
}

/// After `changes`, opening the tunnel of `setup` is 503 `GATEWAY_ERROR` with `message`,
/// and guacd is never reached.
async fn unread(pool: &PgPool, setup: Setup, changes: &[&str], message: &str) {
    let mut guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let bench = bench(pool, &guacd.address, setup).await;
    for change in changes {
        sqlx::query(change)
            .bind(docker_host())
            .execute(pool)
            .await
            .unwrap();
    }
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
async fn sec_007_a_device_without_tls_when_tls_is_required_is_503_gateway_error(pool: PgPool) {
    let message = "The device offers only standard RDP security, so the hub could not check \
                   its server certificate. Check the security setting.";
    unread(&pool, Setup::presents(Presents::StandardOnly), &[], message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_an_unreachable_rdp_port_is_503_gateway_error(pool: PgPool) {
    let closed = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let change = format!("update devices.rdp_settings set port = {port} where device_id = $1");
    let message = "The hub could not reach the device's RDP port to check its server certificate.";
    unread(&pool, Setup::default(), &[&change], message).await;
}
