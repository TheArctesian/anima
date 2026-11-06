//! The one session channel (K4 Section 8.4, GW-013): the hub opens one session channel and
//! sends only `pty-req`, `shell` and `window-change` on it, refuses every channel the device
//! opens, and sends the client no secret and no token in any frame (OD-9, SEC-005).

use sqlx::PgPool;

use super::{
    bench::{Setup, bench},
    fake_ssh::Heard,
    session, socket,
};
use crate::sessions::tests::fixture::{LOOPBACK, PASSWORD, ended, ended_once};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_gw_013_the_device_hears_only_pty_shell_and_window_change_on_one_channel(
    pool: PgPool,
) {
    let mut bench = bench(&pool, Setup::default()).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    let mut heard = Vec::new();
    while !heard.contains(&Heard::Shell) {
        heard.push(bench.ssh.next().await);
    }
    client.send_binary(b"id\r").await;
    client.resize(120, 40).await;
    client.send(r#"{"type":"ping"}"#).await;
    bench.ssh.say(b"uid=1000\r\n").await;
    assert_eq!(client.bytes_until(b"\r\n").await, b"uid=1000\r\n");
    assert_eq!(client.close().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
    heard.extend(bench.ssh.heard_so_far());

    let after_sign_in: Vec<&Heard> = heard
        .iter()
        .filter(|heard| !matches!(heard, Heard::Auth(_)))
        .collect();
    assert!(
        matches!(after_sign_in[..], [Heard::Pty { .. }, Heard::Shell, ..]),
        "the PTY, then the shell: {heard:?}"
    );
    let allowed = |heard: &&Heard| {
        matches!(
            heard,
            Heard::Pty { .. } | Heard::Shell | Heard::WindowChange { .. } | Heard::Input(_)
        )
    };
    assert!(after_sign_in.iter().all(allowed), "{heard:?}");
    let ledger = bench.ssh.ledger.lock().unwrap();
    let [("session", channel)] = ledger.opened[..] else {
        panic!("one session channel: {ledger:?}");
    };
    assert_eq!(ledger.requested_on.len(), 3, "{ledger:?}");
    assert!(
        ledger.requested_on.iter().all(|on| *on == channel),
        "{ledger:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_4_every_channel_the_device_opens_is_refused(pool: PgPool) {
    let (mut bench, mut client) = session(&pool, Setup::default()).await;
    for (kind, opened) in ["session", "x11", "agent", "direct-tcpip", "forwarded-tcpip"]
        .into_iter()
        .chain(["direct-streamlocal", "forwarded-streamlocal"])
        .zip(bench.ssh.open_channels().await)
    {
        assert!(opened.is_err(), "the hub refuses a {kind} channel");
    }
    for _ in 0..2_000 {
        assert!(
            !bench.ssh.open_x11().await,
            "the hub refuses every X11 channel"
        );
    }
    bench.ssh.say(b"still here").await;
    assert_eq!(client.bytes_until(b"still here").await, b"still here");
    assert_eq!(client.close().await, Some(1000));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        LOOPBACK,
    );
    let heard = bench.ssh.heard_so_far();
    assert!(
        !heard
            .iter()
            .any(|heard| matches!(heard, Heard::Forbidden(_))),
        "{heard:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn od_9_sec_005_no_frame_to_the_client_holds_the_password_or_the_token(pool: PgPool) {
    let (bench, mut client) = session(&pool, Setup::default()).await;
    client.send_binary(b"echo hi\r").await;
    bench.ssh.say(b"hi\r\n").await;
    client.bytes_until(b"hi\r\n").await;
    client.send(r#"{"type":"ping"}"#).await;
    bench.ssh.exit().await;
    assert_eq!(
        client.ended().await,
        ("REMOTE_CLOSED".to_owned(), Some(1000))
    );
    let received = String::from_utf8_lossy(&client.received).into_owned();
    assert!(received.contains("hi\r\n"), "the capture works: {received}");
    for secret in [PASSWORD, bench.token.as_str()] {
        assert!(!received.contains(secret), "{secret} reached the client");
    }
}
