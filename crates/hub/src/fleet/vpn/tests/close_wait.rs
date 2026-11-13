//! After the reply line the hub keeps the turn until the broker closes the connection, at most
//! the close wait and never past the call limit, so systemd's `MaxConnections=1` does not close
//! the next call's connection (BD-15). A broker that does not close fails no call. Time is
//! paused, so the waits take no real time.

use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::UnixListener,
    time::Instant,
};

use crate::fleet::vpn::{
    Error,
    broker::{Broker, CALL_LIMIT, CLOSE_WAIT},
};

const KEY_REPLY: &str =
    r#"{"ok":true,"value":{"key":"REDACTED","expiresAt":"2030-05-05T13:00:00.000Z","keyId":7}}"#;

/// A broker that replies after `delay`, then keeps the connection open for `open` (forever
/// when `None`) before it closes.
fn broker_closing(folder: &tempfile::TempDir, delay: Duration, open: Option<Duration>) -> Broker {
    let path = folder.path().join("broker.sock");
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        tokio::time::sleep(delay).await;
        let reply = format!("{KEY_REPLY}\n");
        stream.get_mut().write_all(reply.as_bytes()).await.unwrap();
        match open {
            Some(open) => tokio::time::sleep(open).await,
            None => std::future::pending().await,
        }
    });
    Broker::new(path)
}

#[tokio::test(start_paused = true)]
async fn bd_15_the_turn_is_held_until_the_broker_closes() {
    let folder = tempfile::tempdir().unwrap();
    let open = Duration::from_secs(1);
    let broker = broker_closing(&folder, Duration::ZERO, Some(open));

    let started = Instant::now();
    let call = tokio::spawn({
        let broker = broker.clone();
        async move { broker.personal_key().await.map(drop) }
    });
    tokio::time::sleep(open / 2).await;
    let meanwhile = broker.nodes_if_free().await;
    let answer = call.await.unwrap();

    assert!(matches!(meanwhile, Err(Error::BrokerBusy)), "{meanwhile:?}");
    assert!(answer.is_ok(), "{answer:?}");
    let held = started.elapsed();
    assert!(held >= open && held < CLOSE_WAIT, "{held:?}");
}

#[tokio::test(start_paused = true)]
async fn bd_15_a_broker_that_never_closes_holds_the_turn_only_for_the_close_wait() {
    let folder = tempfile::tempdir().unwrap();
    let broker = broker_closing(&folder, Duration::ZERO, None);

    let started = Instant::now();
    let answer = broker.personal_key().await;

    assert!(answer.is_ok(), "the reply is kept: {:?}", answer.err());
    assert_eq!(started.elapsed(), CLOSE_WAIT);
}

#[tokio::test(start_paused = true)]
async fn bd_10_the_close_wait_never_runs_past_the_call_limit() {
    let folder = tempfile::tempdir().unwrap();
    let late = CALL_LIMIT.checked_sub(CLOSE_WAIT / 2).unwrap();
    let broker = broker_closing(&folder, late, None);

    let started = Instant::now();
    let answer = broker.personal_key().await;

    assert!(answer.is_ok(), "the reply is kept: {:?}", answer.err());
    assert_eq!(started.elapsed(), CALL_LIMIT);
}
