//! A device that stops taking what the hub sends it (K4 Section 8.5): client input waits for
//! the device under the channel's window, one write at a time and in order, with the hub's
//! pings going on and the client's silence not counted; a write or a PTY request that waits
//! past the request wait ends the session `GATEWAY_ERROR`, unless `endSession` comes first.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    super::super::route::Terminal,
    WAIT,
    bench::{Setup, bench},
    fake_ssh::{Heard, Hold, Script},
    session,
    socket::{self, Received},
};
use crate::sessions::{
    Actor,
    end::end_session,
    tests::fixture::{LOOPBACK, address, ended, ended_once, now, operator},
};

/// The default setup with the server holding `hold` and the route's waits tuned by `tune`.
fn holding(hold: Hold, tune: impl FnOnce(&mut Terminal) + Send + 'static) -> Setup {
    Setup {
        script: Script {
            hold,
            ..Script::default()
        },
        ..Setup::terminal(tune)
    }
}

/// Input bytes the server has read so far, waiting until there are `len` of them.
async fn input_until(bench: &mut super::Bench, len: usize) -> Vec<u8> {
    let mut input = Vec::new();
    while input.len() < len {
        match bench.ssh.next().await {
            Heard::Input(bytes) => input.extend(bytes),
            other => panic!("{other:?} while input was due"),
        }
    }
    input
}

/// A message of `len` bytes no two neighbouring stretches of which are alike.
fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_device_that_stops_taking_input_is_gateway_error_and_pings_go_on(pool: PgPool) {
    // The client's idle wait is shorter than the request wait: if the time the input waits
    // on the device counted as the client's silence, the session would end `TIMEOUT`.
    let setup = holding(Hold::Input, |terminal| {
        terminal.timing.hub_silence = Duration::from_millis(200);
        terminal.timing.client_idle = Duration::from_millis(500);
        terminal.timing.request = Duration::from_millis(2_000);
    });
    let (bench, mut client) = session(&pool, setup).await;
    client.send_binary(&pattern(16 * 1024)).await;
    let mut pings = 0;
    let reason = loop {
        match client.next().await {
            Received::Text(text) if text == r#"{"type":"ping"}"# => pings += 1,
            Received::Text(text) => break text,
            other => panic!("{other:?} while the write waited"),
        }
    };
    assert_eq!(reason, r#"{"type":"end","reason":"GATEWAY_ERROR"}"#);
    assert_eq!(client.close_code().await, Some(1011));
    assert!(
        pings >= 5,
        "the hub pinged {pings} times while the write waited"
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The device stopped reading what the operator typed, so the hub closed the SSH session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_one_write_at_a_time_keeps_client_input_whole_and_in_order(pool: PgPool) {
    let setup = holding(Hold::Input, |_| {});
    let (mut bench, mut client) = session(&pool, setup).await;
    let first = pattern(16 * 1024);
    let second = b"second message".to_vec();
    client.send_binary(&first).await;
    client.send_binary(&second).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let held: usize = bench
        .ssh
        .heard_so_far()
        .iter()
        .map(|heard| match heard {
            Heard::Input(bytes) => bytes.len(),
            other => panic!("{other:?} while the write waited"),
        })
        .sum();
    assert!(
        held < first.len(),
        "the device holds the first write: {held}"
    );
    bench.ssh.release();
    let rest = input_until(&mut bench, first.len() + second.len() - held).await;
    let mut expected = first;
    expected.extend(&second);
    assert_eq!(rest[..], expected[held..], "both whole, in order");
    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_pty_request_the_device_does_not_answer_is_gateway_error(pool: PgPool) {
    let setup = holding(Hold::Pty, |terminal| {
        terminal.timing.hub_silence = Duration::from_millis(100);
        terminal.timing.request = Duration::from_millis(600);
    });
    let mut bench = bench(&pool, setup).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    assert!(matches!(
        bench.ssh.next_after_sign_in().await,
        Heard::Pty { .. }
    ));
    let mut pings = 0;
    let reason = loop {
        match client.next().await {
            Received::Text(text) if text == r#"{"type":"ping"}"# => pings += 1,
            Received::Text(text) => break text,
            other => panic!("{other:?} while the PTY request waited"),
        }
    };
    assert_eq!(reason, r#"{"type":"end","reason":"GATEWAY_ERROR"}"#);
    assert_eq!(client.close_code().await, Some(1011));
    assert!(
        pings >= 2,
        "the hub pinged {pings} times while the request waited"
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The device did not answer the hub's request for a terminal or shell in time, so the \
         hub closed the SSH session."
    );
    bench.ssh.release();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_10_end_session_while_the_pty_request_waits_is_operator(pool: PgPool) {
    let mut bench = bench(&pool, holding(Hold::Pty, |_| {})).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    assert!(matches!(
        bench.ssh.next_after_sign_in().await,
        Heard::Pty { .. }
    ));
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let row = tokio::time::timeout(WAIT, end_session(&pool, &bench.live, bench.session, &by))
        .await
        .unwrap()
        .unwrap();
    assert!(row.end.is_some(), "the session ended its row in time");
    assert_eq!(client.ended().await, ("OPERATOR".to_owned(), Some(1000)));
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
    assert!(
        !bench.ssh.heard_so_far().contains(&Heard::Shell),
        "no shell after the end"
    );
}
