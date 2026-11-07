//! `GET /files` (K4 Sections 8.1 and 9): the refusals before the upgrade, and the SSH
//! requests the device sees: one session channel and the `sftp` subsystem, nothing else.

use axum::http::StatusCode;
use sqlx::PgPool;

use super::{
    bench::bench,
    server::{Heard, serve},
    socket::{open, open_with},
};
use crate::sessions::{
    files::{tickets::FilesTicket, trail::HOST_KEY_CHANGED},
    start,
    tests::fixture::{LOOPBACK, address, docker_host, move_device, now, operator, request},
    tickets::{LIFETIME, TicketToken},
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_the_device_sees_only_the_sftp_subsystem(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(client.text().await["type"], "entries");
    assert_eq!(client.close().await, Some(1000));
    assert_eq!(
        bench.server.requests(),
        [Heard::Session, Heard::Subsystem("sftp".to_owned())]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_a_files_ticket_opens_once(pool: PgPool) {
    let bench = bench(&pool).await;
    let token = bench.token().await;
    let mut client = open(&bench.url(&token)).await;
    assert_eq!(client.close().await, Some(1000));
    let again = open_with(&bench.url(&token), true).await;
    assert_eq!(again.err(), Some(StatusCode::FORBIDDEN));
    let none = open_with(&format!("ws://{}/files", bench.served), true).await;
    assert_eq!(none.err(), Some(StatusCode::FORBIDDEN));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_a_ticket_for_another_address_is_refused_and_spent(pool: PgPool) {
    let bench = bench(&pool).await;
    let token = TicketToken::from_random([42; 32]);
    let ticket = FilesTicket {
        operator: operator(),
        client: address("127.0.0.2"),
        session: bench.session,
        device: docker_host(),
        expires_at: now() + LIFETIME,
    };
    bench
        .live
        .files()
        .issue(token.hash(), ticket, now())
        .unwrap();
    let refused = open_with(&bench.url(token.expose()), true).await;
    assert_eq!(refused.err(), Some(StatusCode::FORBIDDEN));
    assert_eq!(bench.live.files().len(), 0, "spent");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_a_session_ticket_does_not_open_files(pool: PgPool) {
    let bench = bench(&pool).await;
    move_device(&pool, "10.10.0.21", 22).await;
    let clock: std::sync::Arc<dyn crate::clock::Clock> =
        std::sync::Arc::new(crate::clock::FixedClock(now()));
    let client = address(LOOPBACK);
    let session = start(&pool, &clock, &bench.live, &operator(), client, request())
        .await
        .unwrap();
    let refused = open_with(&bench.url(session.token()), true).await;
    assert_eq!(refused.err(), Some(StatusCode::FORBIDDEN));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_ticket_for_an_ended_session_is_refused(pool: PgPool) {
    let mut bench = bench(&pool).await;
    let token = bench.token().await;
    drop(bench.terminal.take());
    let refused = open_with(&bench.url(&token), true).await;
    assert_eq!(refused.err(), Some(StatusCode::FORBIDDEN));
    assert_eq!(bench.server.requests(), [], "no link opened");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_without_the_subprotocol_400_and_the_ticket_is_spent(pool: PgPool) {
    let bench = bench(&pool).await;
    let token = bench.token().await;
    let plain = open_with(&bench.url(&token), false).await;
    assert_eq!(plain.err(), Some(StatusCode::BAD_REQUEST));
    let again = open_with(&bench.url(&token), true).await;
    assert_eq!(again.err(), Some(StatusCode::FORBIDDEN));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_an_unreachable_device_is_503(pool: PgPool) {
    let bench = bench(&pool).await;
    let closed = std::net::TcpListener::bind((LOOPBACK, 0)).unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    move_device(&pool, LOOPBACK, port).await;
    let token = bench.token().await;
    let refused = open_with(&bench.url(&token), true).await;
    assert_eq!(refused.err(), Some(StatusCode::SERVICE_UNAVAILABLE));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_the_deny_list_refuses_the_device(pool: PgPool) {
    let bench = super::bench::bench_with(&pool, |files| {
        files.policy = crate::targets::AddressPolicy::default();
    })
    .await;
    let token = bench.token().await;
    let refused = open_with(&bench.url(&token), true).await;
    assert_eq!(refused.err(), Some(StatusCode::SERVICE_UNAVAILABLE));
    assert_eq!(bench.server.heard(), [], "no connection reached the device");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_files_ticket_presented_after_shutdown_is_refused(pool: PgPool) {
    let bench = bench(&pool).await;
    let token = bench.token().await;
    let _closing = bench.live.shut_down();
    let refused = open_with(&bench.url(&token), true).await;
    assert_eq!(refused.err(), Some(StatusCode::FORBIDDEN));
    assert_eq!(bench.server.requests(), [], "no link opened");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_another_host_key_is_503_with_an_error_event(pool: PgPool) {
    let bench = bench(&pool).await;
    let other = serve(12).await.key;
    sqlx::query(
        "update sessions.host_keys set algorithm = $1, fingerprint_sha256 = $2, public_key = $3",
    )
    .bind(other.algorithm())
    .bind(other.fingerprint().as_str().to_owned())
    .bind(other.to_base64())
    .execute(&pool)
    .await
    .unwrap();
    let token = bench.token().await;
    let refused = open_with(&bench.url(&token), true).await;
    assert_eq!(refused.err(), Some(StatusCode::SERVICE_UNAVAILABLE));
    assert_eq!(bench.server.heard(), [], "no sign-in, no channel");
    let events = bench.events().await;
    let (severity, _, device, message) = events.last().unwrap();
    assert_eq!(
        (severity.as_str(), device.as_deref(), message.as_str()),
        ("error", Some("docker-host"), HOST_KEY_CHANGED)
    );
}
