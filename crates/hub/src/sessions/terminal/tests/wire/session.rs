//! One full terminal session (K4 Sections 7 and 8.4): the PTY at the client's size, bytes
//! both ways in order (standard error included), `resize` as a window change, `ping` both
//! ways, and the shell's exit as `REMOTE_CLOSED`.

use std::time::Duration;

use sqlx::PgPool;

use super::{
    WAIT,
    bench::{Setup, bench},
    fake_ssh::Heard,
    session,
    socket::{self, Received},
};
use crate::sessions::tests::fixture::{LOOPBACK, ended, ended_once, stored_text};

/// Everything the operator typed that reached the server, until it holds `expected`.
async fn typed_until(bench: &mut super::Bench, expected: &[u8]) -> Vec<u8> {
    let mut typed = Vec::new();
    while !typed
        .windows(expected.len())
        .any(|window| window == expected)
    {
        match bench.ssh.next().await {
            Heard::Input(bytes) => typed.extend(bytes),
            other => panic!("{other:?} while input was due"),
        }
    }
    typed
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_ra_011_a_full_session_relays_bytes_both_ways_and_resizes(pool: PgPool) {
    let mut bench = bench(&pool, Setup::default()).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(132, 43).await;
    let pty = bench.ssh.next_after_sign_in().await;
    let expected = Heard::Pty {
        term: "xterm-256color".to_owned(),
        cols: 132,
        rows: 43,
        pixels: (0, 0),
        modes: 0,
    };
    assert_eq!(
        pty, expected,
        "K4 Section 8.4: the PTY at the client's size"
    );
    assert_eq!(bench.ssh.next().await, Heard::Shell);

    bench.ssh.say(b"first ").await;
    bench.ssh.complain(b"second ").await;
    bench.ssh.say(b"third\x1b]7;file://host/tmp\x07").await;
    let output = client.bytes_until(b"\x07").await;
    assert_eq!(
        output, b"first second third\x1b]7;file://host/tmp\x07",
        "in order, unchanged"
    );

    let typed: &[u8] = b"ls -la\r\x03\x1b[A\xff\x00";
    client.send_binary(typed).await;
    assert_eq!(
        typed_until(&mut bench, typed).await,
        typed,
        "unchanged, any bytes"
    );

    client.resize(100, 30).await;
    let change = Heard::WindowChange {
        cols: 100,
        rows: 30,
        pixels: (0, 0),
    };
    assert_eq!(bench.ssh.next().await, change);

    bench.ssh.exit().await;
    assert_eq!(
        client.ended().await,
        ("REMOTE_CLOSED".to_owned(), Some(1000))
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "remote_closed", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The shell exited or the device closed the SSH session."
    );
    let stored = stored_text(&pool).await;
    for content in ["first", "second", "ls -la", "file://host"] {
        assert!(!stored.contains(content), "RA-008: {content} is not kept");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_the_hub_pings_when_silent_and_client_pings_keep_the_session(pool: PgPool) {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.hub_silence = Duration::from_millis(200);
        terminal.timing.client_idle = Duration::from_millis(1_500);
    });
    let (bench, mut client) = session(&pool, setup).await;
    // The client pings for three of its idle waits; the hub pings it meanwhile.
    let mut hub_pings = 0;
    for _ in 0..15 {
        client.send(r#"{"type":"ping"}"#).await;
        let started = tokio::time::Instant::now();
        while started.elapsed() < Duration::from_millis(300) {
            let next = tokio::time::timeout(Duration::from_millis(300), client.next()).await;
            match next {
                Ok(Received::Text(text)) if text == r#"{"type":"ping"}"# => hub_pings += 1,
                Ok(other) => panic!("{other:?} while only pings were due"),
                Err(_) => {}
            }
        }
    }
    assert!(hub_pings >= 5, "the hub pinged {hub_pings} times");
    bench.ssh.say(b"still here").await;
    assert_eq!(client.bytes_until(b"still here").await, b"still here");
    assert_eq!(client.close().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_output_the_client_has_not_read_waits_in_the_device_not_the_hub(pool: PgPool) {
    // The client reads nothing while the shell writes far more than the hub may hold: the
    // shell's writes stall (its window and the hub's channel buffer are full), and once the
    // client reads, every byte arrives in order.
    let (bench, mut client) = session(&pool, Setup::default()).await;
    let chunk = vec![b'x'; 32 * 1024];
    let ssh = std::sync::Arc::new(bench.ssh);
    let writer = std::sync::Arc::clone(&ssh);
    let flood = tokio::spawn(async move {
        for _ in 0..512 {
            writer.say(&chunk).await;
        }
        writer.say(b"end of flood").await;
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !flood.is_finished(),
        "16 MiB do not fit between the shell and a stalled client"
    );
    let mut total = 0;
    loop {
        match tokio::time::timeout(WAIT, client.next()).await.unwrap() {
            Received::Binary(bytes) => {
                total += bytes.len();
                if bytes.ends_with(b"end of flood") {
                    break;
                }
            }
            other => panic!("{other:?} during the flood"),
        }
    }
    assert_eq!(total, 512 * 32 * 1024 + b"end of flood".len());
    flood.await.unwrap();
}
