//! One full terminal session (K4 Sections 7 and 8.4): the PTY at the client's size, bytes
//! both ways in order (standard error included), `resize` as a window change, `ping` both
//! ways, and the shell's exit as `REMOTE_CLOSED`.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use sqlx::PgPool;

use super::{
    WAIT,
    bench::{Setup, bench},
    fake_ssh::Heard,
    session,
    socket::{self, Received},
};
use crate::sessions::{
    EndReason,
    end::shut_down,
    tests::fixture::{LOOPBACK, ended, ended_once, now, stored_text},
};

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
    // The client pings five times per idle wait, the hub every 200 ms of its own silence.
    let setup = Setup::terminal(|terminal| {
        terminal.timing.hub_silence = Duration::from_millis(200);
        terminal.timing.client_idle = Duration::from_millis(1_500);
    });
    let (bench, mut client) = session(&pool, setup).await;
    // The client pings for three of its idle waits; the hub pings it meanwhile, some 20
    // times in all.
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
    assert!(hub_pings >= 3, "the hub pinged {hub_pings} times");
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
    // The client reads nothing while the shell writes until its writes stall (its window and
    // the hub's channel buffer are full); once the client reads, every byte arrives in order.
    let (bench, mut client) = session(&pool, Setup::default()).await;
    let chunk = vec![b'x'; 32 * 1024];
    let ssh = Arc::new(bench.ssh);
    let (writer, written, stop) = (
        Arc::clone(&ssh),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let (counted, stopped) = (Arc::clone(&written), Arc::clone(&stop));
    let flood = tokio::spawn(async move {
        while !stopped.load(Ordering::SeqCst) {
            writer.say(&chunk).await;
            counted.fetch_add(chunk.len(), Ordering::SeqCst);
        }
        writer.say(b"end of flood").await;
    });
    let stalled_at = loop {
        let before = written.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let after = written.load(Ordering::SeqCst);
        if before == after && after > 0 {
            break after;
        }
        assert!(after < 64 * 1024 * 1024, "the shell's writes never stall");
    };
    assert!(
        stalled_at < 16 * 1024 * 1024,
        "{stalled_at} bytes went out before the shell stalled"
    );
    stop.store(true, Ordering::SeqCst);
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
    flood.await.unwrap();
    let written = written.load(Ordering::SeqCst);
    assert_eq!(total, written + b"end of flood".len());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_034_shutdown_publishes_the_terminal_s_hub_shutdown_end_once_written(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    let mut changes = bench.live.changes();

    shut_down(&pool, &bench.live, now()).await.unwrap();

    // The tunnel publishes before it deregisters, and shutdown waits for that.
    let change = changes.try_recv().expect("the end was published");
    assert_eq!(change.session.id, bench.session);
    assert_eq!(
        change.session.end.map(|end| end.reason),
        Some(EndReason::HubShutdown)
    );
    assert!(changes.try_recv().is_err(), "one change for one end");
    ended_once(
        &ended(&pool, bench.session).await,
        "hub_shutdown",
        "info",
        LOOPBACK,
    );
    assert_eq!(
        client.ended().await,
        ("HUB_SHUTDOWN".to_owned(), Some(1001))
    );
}
