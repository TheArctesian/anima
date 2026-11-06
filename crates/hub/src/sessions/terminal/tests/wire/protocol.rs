//! Client messages outside K4 Section 8.4 or over a limit of Section 8.5 end the session
//! `PROTOCOL_ERROR` with 1002, after the hub's `end`, and never reach the device.

use sqlx::PgPool;

use super::{
    Bench,
    bench::{Setup, bench},
    fake_ssh::Heard,
    session,
    socket::{self, Socket},
};
use crate::sessions::tests::fixture::{LOOPBACK, ended, ended_once};

/// The session ended `PROTOCOL_ERROR`: the client heard `end` and 1002, the row, audit entry
/// and event are written once, and nothing the client sent reached the device.
async fn broke_the_protocol(pool: &PgPool, mut bench: Bench, mut client: Socket) {
    assert_eq!(
        client.ended().await,
        ("PROTOCOL_ERROR".to_owned(), Some(1002))
    );
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "protocol_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client sent a message the hub does not accept, so the hub closed the SSH session."
    );
    let heard = bench.ssh.heard_so_far();
    assert!(
        !heard
            .iter()
            .any(|heard| matches!(heard, Heard::Input(_) | Heard::WindowChange { .. })),
        "{heard:?}"
    );
}

/// After the PTY and shell, the client sends `text`.
async fn after_resize(pool: &PgPool, text: &str) {
    let (bench, mut client) = session(pool, Setup::default()).await;
    client.send(text).await;
    broke_the_protocol(pool, bench, client).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_text_that_is_not_json_is_protocol_error(pool: PgPool) {
    after_resize(&pool, "resize 80 24").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_an_unknown_type_is_protocol_error(pool: PgPool) {
    after_resize(&pool, r#"{"type":"exec","command":"id"}"#).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_gw_013_an_extra_field_is_protocol_error(pool: PgPool) {
    after_resize(
        &pool,
        r#"{"type":"resize","cols":80,"rows":24,"term":"vt100"}"#,
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_a_size_out_of_range_is_protocol_error(pool: PgPool) {
    after_resize(&pool, r#"{"type":"resize","cols":1001,"rows":24}"#).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_text_over_256_bytes_is_protocol_error(pool: PgPool) {
    let padded = format!(r#"{{"type":"ping"}}{}"#, " ".repeat(250));
    after_resize(&pool, &padded).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_binary_message_over_64_kib_is_protocol_error(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    client.send_binary(&vec![b'a'; 64 * 1024 + 1]).await;
    broke_the_protocol(&pool, bench, client).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_message_over_64_kib_across_fragments_is_protocol_error(pool: PgPool) {
    // Every frame is well under the frame cap; only the whole message is over.
    let (bench, mut client) = session(&pool, Setup::default()).await;
    client.send_fragmented(&vec![b'a'; 64 * 1024 + 1], 5).await;
    broke_the_protocol(&pool, bench, client).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_message_of_64_kib_across_fragments_reaches_the_device_whole(pool: PgPool) {
    let (mut bench, mut client) = session(&pool, Setup::default()).await;
    let typed = vec![b'a'; 64 * 1024];
    client.send_fragmented(&typed, 4).await;
    let mut input = Vec::new();
    while input.len() < typed.len() {
        match bench.ssh.next().await {
            Heard::Input(bytes) => input.extend(bytes),
            other => panic!("{other:?} while input was due"),
        }
    }
    assert_eq!(input, typed);
    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_text_that_is_not_utf_8_is_protocol_error(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    client.send_unchecked_text(b"{\"type\":\"ping\xff\"}").await;
    broke_the_protocol(&pool, bench, client).await;
}

/// Before any size, the client sends `send`'s message.
async fn before_resize(pool: &PgPool, binary: bool) {
    let mut bench = bench(pool, Setup::default()).await;
    let mut client = socket::open(&bench.url).await;
    if binary {
        client.send_binary(b"ls\r").await;
    } else {
        client.send(r#"{"type":"ping"}"#).await;
    }
    let heard = bench.ssh.heard_so_far();
    assert!(!heard.contains(&Heard::Shell), "{heard:?}");
    broke_the_protocol(pool, bench, client).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_bytes_before_the_first_resize_are_protocol_error(pool: PgPool) {
    before_resize(&pool, true).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_a_ping_before_the_first_resize_is_protocol_error(pool: PgPool) {
    before_resize(&pool, false).await;
}
