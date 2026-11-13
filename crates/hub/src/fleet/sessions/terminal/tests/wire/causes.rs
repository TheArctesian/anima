//! Each end cause of K4 Section 8.6 on an open terminal: the client's `end` message, the
//! close code, and one row end, audit entry and event.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::{
    task::JoinHandle,
    time::{Instant, sleep, sleep_until, timeout},
};

use super::{
    bench::{Setup, bench},
    fake_ssh::{FakeSsh, Heard, Script},
    session,
    socket::{self, Received},
};
use crate::fleet::sessions::{
    Actor, EndReason,
    end::end_session,
    tests::fixture::{LOOPBACK, address, ended, ended_once, now, operator},
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_a_client_close_is_operator_1000(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    assert_eq!(client.close().await, Some(1000));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
    assert_eq!(ended.events[0].3, "The operator closed the SSH session.");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_d_10_end_session_is_operator_1000_and_names_who_asked(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let row = end_session(&pool, &bench.live, bench.session, &by)
        .await
        .unwrap();
    assert_eq!(row.end.map(|end| end.reason), Some(EndReason::Operator));
    assert_eq!(client.ended().await, ("OPERATOR".to_owned(), Some(1000)));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_d_10_hub_shutdown_is_1001_and_finishes(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    let (closing, _) = bench.live.shut_down();
    assert_eq!(
        client.ended().await,
        ("HUB_SHUTDOWN".to_owned(), Some(1001))
    );
    timeout(Duration::from_secs(5), closing.finished())
        .await
        .expect("the session deregisters");
    ended_once(
        &ended(&pool, bench.session).await,
        "hub_shutdown",
        "info",
        LOOPBACK,
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_a_device_connection_that_drops_is_gateway_error_1011(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    bench.ssh.cut();
    assert_eq!(
        client.ended().await,
        ("GATEWAY_ERROR".to_owned(), Some(1011))
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The SSH connection to the device failed, so the hub closed the session."
    );
}

/// A session whose server refuses the PTY or the shell, as `script` says: the client hears
/// `GATEWAY_ERROR` and 1011, and the event says which was refused.
async fn refused(pool: &PgPool, script: Script, message: &str) {
    let mut bench = bench(pool, Setup::script(script)).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    assert!(matches!(
        bench.ssh.next_after_sign_in().await,
        Heard::Pty { .. }
    ));
    assert_eq!(
        client.ended().await,
        ("GATEWAY_ERROR".to_owned(), Some(1011))
    );
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(ended.events[0].3, message);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_a_refused_pty_is_gateway_error_1011(pool: PgPool) {
    let script = Script {
        refuse_pty: true,
        ..Script::default()
    };
    let message =
        "The device refused a terminal for the session, so the hub closed the SSH session.";
    refused(&pool, script, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_a_refused_shell_is_gateway_error_1011(pool: PgPool) {
    let script = Script {
        refuse_shell: true,
        ..Script::default()
    };
    let message = "The device refused to start a shell, so the hub closed the SSH session.";
    refused(&pool, script, message).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_no_first_resize_in_time_is_timeout_4002(pool: PgPool) {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.first_size = Duration::from_millis(300);
    });
    let mut bench = bench(&pool, setup).await;
    let mut client = socket::open(&bench.url).await;
    assert_eq!(client.ended().await, ("TIMEOUT".to_owned(), Some(4002)));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "timeout", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client did not send its terminal size in time, so the hub closed the SSH session."
    );
    let heard = bench.ssh.heard_so_far();
    assert!(
        !heard
            .iter()
            .any(|heard| matches!(heard, Heard::Pty { .. } | Heard::Shell)),
        "no PTY before the size: {heard:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_client_silence_is_timeout_4002(pool: PgPool) {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.client_idle = Duration::from_millis(500);
    });
    let (bench, mut client) = session(&pool, setup).await;
    assert_eq!(client.ended().await, ("TIMEOUT".to_owned(), Some(4002)));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "timeout", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client sent nothing for too long, so the hub closed the SSH session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_client_ping_sent_while_the_hub_s_send_waits_keeps_the_session(pool: PgPool) {
    // Each round: the client pings and waits until the hub has read it (`read`); a second
    // later the shell floods until the hub's send to the unread client waits; only then does
    // the client ping again, unread. It reads just after `read` plus the idle wait, so the
    // idle wait has run out, and a second before the waiting send, which began after the
    // flood, reaches its own limit of the same length. The second ping is waiting to be
    // read: it, not the idle wait, must win. When both are ready the hub picks one at
    // random, so without the peek such a round ends the session half the time. A round in
    // which the connection lets the hub's send through before the idle wait runs out has the
    // hub read the second ping at once; it passes without testing the peek, so there are
    // twelve rounds.
    let idle = Duration::from_secs(2);
    let setup = Setup::terminal(move |terminal| terminal.timing.client_idle = idle);
    let (bench, mut client) = session(&pool, setup).await;
    let ssh = Arc::new(bench.ssh);
    for round in 0..12 {
        client.send(r#"{"type":"ping"}"#).await;
        client.caught_up().await;
        let read = Instant::now();
        sleep(Duration::from_secs(1)).await;
        let marker = format!("end of round {round}");
        let flood = Flood::start(&ssh, marker.clone());
        flood.held().await;
        client.send(r#"{"type":"ping"}"#).await;
        sleep_until(read + idle + Duration::from_millis(100)).await;
        loop {
            match client.next().await {
                Received::Binary(bytes) if bytes.ends_with(marker.as_bytes()) => break,
                Received::Binary(_) => {}
                Received::Text(text) if text == r#"{"type":"ping"}"# => {}
                other => panic!("{other:?} in round {round}"),
            }
        }
        flood.task.await.unwrap();
    }
    ssh.say(b"still here").await;
    assert_eq!(client.bytes_until(b"still here").await, b"still here");
    assert_eq!(client.close().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
}

/// The shell writing to a client that does not read: chunks until told to stop, then a
/// marker.
struct Flood {
    task: JoinHandle<()>,
    written: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl Flood {
    fn start(ssh: &Arc<FakeSsh>, marker: String) -> Self {
        let (writer, written, stop) = (
            Arc::clone(ssh),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicBool::new(false)),
        );
        let (counted, stopped) = (Arc::clone(&written), Arc::clone(&stop));
        let task = tokio::spawn(async move {
            let chunk = vec![b'x'; 32 * 1024];
            while !stopped.load(Ordering::SeqCst) {
                writer.say(&chunk).await;
                counted.fetch_add(chunk.len(), Ordering::SeqCst);
            }
            writer.say(marker.as_bytes()).await;
        });
        Self {
            task,
            written,
            stop,
        }
    }

    /// Returns once the shell has written something and then nothing for 800 ms, as when the
    /// hub's send to the client waits, and tells the shell to write only the marker after
    /// the chunk it is on, so what is left to read is what the hub holds. A loopback
    /// connection that has just filled often lets one more send through within about 600 ms;
    /// the wait outlasts that.
    async fn held(&self) {
        let (mut seen, mut since) = (0, Instant::now());
        loop {
            sleep(Duration::from_millis(50)).await;
            let written = self.written.load(Ordering::SeqCst);
            if written != seen {
                (seen, since) = (written, Instant::now());
            } else if written > 0 && since.elapsed() >= Duration::from_millis(800) {
                self.stop.store(true, Ordering::SeqCst);
                return;
            }
        }
    }
}
