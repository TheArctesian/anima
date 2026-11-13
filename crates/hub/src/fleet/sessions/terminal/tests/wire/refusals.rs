//! Refusals before the upgrade (K4 Section 8.1): 403 for every ticket problem, 400 without the
//! subprotocol, 503 when the device cannot be opened; and the token and the password never
//! reach the process log (LOG-007).

use std::sync::Arc;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::{
    bench::{Setup, bench},
    session,
    socket::open_with,
};
use crate::{
    clock::{Clock, FixedClock},
    fleet::{
        events::process_log,
        sessions::{
            Live,
            tests::fixture::{
                Capture, LOOPBACK, PASSWORD, address, docker_host, ended, ended_once, move_device,
                now,
            },
        },
    },
};

/// Whether the session's row is still live.
async fn is_live(pool: &PgPool, bench: &super::Bench) -> bool {
    crate::fleet::sessions::store::get(pool, bench.session)
        .await
        .unwrap()
        .unwrap()
        .is_live()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_api_009_no_or_unknown_token_is_403_and_spends_nothing(pool: PgPool) {
    let bench = bench(&pool, Setup::default()).await;
    let base = bench.url.split('?').next().unwrap().to_owned();
    let unknown = format!("{base}?token={}", "A".repeat(43));
    let tampered = format!("{base}?token={}x", &bench.token[..42]);
    for url in [base.clone(), format!("{base}?token="), unknown, tampered] {
        assert_eq!(
            open_with(&url, true).await.err(),
            Some(StatusCode::FORBIDDEN),
            "{url}"
        );
    }
    assert!(
        is_live(&pool, &bench).await,
        "the session waits for its ticket"
    );
    assert!(
        open_with(&bench.url, true).await.is_ok(),
        "the ticket still opens"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_api_009_a_used_ticket_is_403(pool: PgPool) {
    let (bench, _client) = session(&pool, Setup::default()).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert!(
        is_live(&pool, &bench).await,
        "the open session is untouched"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_api_009_an_expired_ticket_is_403_and_swept_timeout(pool: PgPool) {
    let setup = Setup::terminal(|terminal| {
        let later: Arc<dyn Clock> = Arc::new(FixedClock(now() + TimeDelta::seconds(30)));
        terminal.clock = later;
    });
    let bench = bench(&pool, setup).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "timeout");
    assert!(
        ended.events.is_empty(),
        "no tunnel opened, so no event (D-13)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_sec_002_a_ticket_from_another_address_is_403(pool: PgPool) {
    let setup = Setup {
        ticket_client: address("10.10.0.77"),
        ..Setup::default()
    };
    let bench = bench(&pool, setup).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_no_terminal_subprotocol_is_400_spends_the_ticket_and_is_protocol_error(
    pool: PgPool,
) {
    let mut bench = bench(&pool, Setup::default()).await;
    assert_eq!(
        open_with(&bench.url, false).await.err(),
        Some(StatusCode::BAD_REQUEST)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "protocol_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client did not open the tunnel as a terminal WebSocket, so the hub closed the SSH \
         session."
    );
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(bench.ssh.heard_so_far(), [], "the device is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_an_unreachable_device_is_503_and_gateway_error(pool: PgPool) {
    let bench = bench(&pool, Setup::default()).await;
    let closed = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    move_device(&pool, LOOPBACK, port).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The hub could not reach the device's SSH port."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_1_ssh_turned_off_after_start_is_503_and_gateway_error(pool: PgPool) {
    let mut bench = bench(&pool, Setup::default()).await;
    sqlx::query("update devices.ssh_settings set enabled = false where device_id = $1")
        .bind(docker_host())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "SSH was turned off for the device before the session opened."
    );
    assert_eq!(bench.ssh.heard_so_far(), [], "the device is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_a_host_that_resolves_to_loopback_is_503_and_never_reached(pool: PgPool) {
    // The hub's own rule, not the tests' loopback exception.
    let setup = Setup {
        live: Live::default(),
        ..Setup::default()
    };
    let mut bench = bench(&pool, setup).await;
    move_device(&pool, "localhost", bench.ssh.port).await;
    assert_eq!(
        open_with(&bench.url, true).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert!(ended.events[0].3.contains("loopback"), "{:?}", ended.events);
    assert_eq!(bench.ssh.heard_so_far(), [], "the device is never reached");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_the_token_and_password_never_reach_the_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in the Guacamole tunnel's test: a second registered dispatcher makes every callsite
    // registration ask all of them, so another test thread cannot disable these callsites.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    let (bench, mut client) = session(&pool, Setup::default()).await;
    client.send_binary(PASSWORD.as_bytes()).await;
    assert_eq!(client.close().await, Some(1000));
    ended(&pool, bench.session).await;
    for subprotocol in [true, false] {
        assert_eq!(
            open_with(&bench.url, subprotocol).await.err(),
            Some(StatusCode::FORBIDDEN)
        );
    }

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("log_007 capture marker"),
        "the capture works: {log}"
    );
    assert!(log.contains("session ended"), "the end is logged: {log}");
    for secret in [bench.token.as_str(), PASSWORD, "token="] {
        assert!(!log.contains(secret), "{secret} in the log");
    }
}
