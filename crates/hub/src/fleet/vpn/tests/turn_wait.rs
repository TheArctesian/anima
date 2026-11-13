//! The broker client's time limits against the broker's own (BD-10, BD-15): a change the
//! broker makes at its last moment reaches the hub, and a call behind a hung broker gives up
//! after the turn wait, having sent nothing. Time is paused, so the waits take no real time.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::UnixListener,
    time::Instant,
};

use crate::fleet::vpn::{
    Error,
    broker::{
        BROKER_LOCK_WAIT, BROKER_REQUEST_DEADLINE, BROKER_REQUEST_READ, BROKER_RUNTIME_MAX, Broker,
        CALL_LIMIT, TURN_WAIT,
    },
    reader::{BROKER_WAIT, read_broker},
};

/// A broker that accepts every connection, reads nothing and never answers; it counts them.
fn hung_broker(folder: &tempfile::TempDir) -> (Broker, Arc<AtomicUsize>) {
    let path = folder.path().join("broker.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counted = accepted.clone();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            held.push(stream);
        }
    });
    (Broker::new(path), accepted)
}

/// Waits until the hung broker holds the first call's connection.
async fn connected(accepted: &AtomicUsize) {
    while accepted.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[tokio::test(start_paused = true)]
async fn bd_15_a_call_behind_a_hung_broker_gives_up_after_the_turn_wait_unsent() {
    let folder = tempfile::tempdir().unwrap();
    let (broker, accepted) = hung_broker(&folder);
    let first = tokio::spawn({
        let broker = broker.clone();
        async move { broker.personal_key().await.map(drop) }
    });
    connected(&accepted).await;

    let started = Instant::now();
    let second = broker.personal_key().await.map(drop);
    let waited = started.elapsed();

    assert!(matches!(second, Err(Error::BrokerBusy)), "{second:?}");
    assert!(waited >= TURN_WAIT && waited < CALL_LIMIT, "{waited:?}");
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        1,
        "the second call sent nothing"
    );
    assert!(matches!(first.await.unwrap(), Err(Error::BrokerTimedOut)));
    assert!(
        started.elapsed() <= CALL_LIMIT,
        "the first call kept its own limit"
    );
}

#[tokio::test(start_paused = true)]
async fn bd_15_the_reader_s_node_read_is_busy_at_once_while_a_call_holds_the_turn() {
    let folder = tempfile::tempdir().unwrap();
    let (broker, accepted) = hung_broker(&folder);
    let first = tokio::spawn({
        let broker = broker.clone();
        async move { broker.personal_key().await.map(drop) }
    });
    connected(&accepted).await;

    let started = Instant::now();
    let read = broker.nodes_if_free().await;

    assert!(matches!(read, Err(Error::BrokerBusy)), "{read:?}");
    assert_eq!(started.elapsed(), Duration::ZERO);
    first.abort();
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
}

/// The broker's worst case: it waited for its lock, for the request line, then made the key
/// at its request deadline.
#[tokio::test(start_paused = true)]
async fn bd_10_a_key_the_broker_makes_at_its_last_moment_reaches_the_hub() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("broker.sock");
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        tokio::time::sleep(BROKER_LOCK_WAIT + BROKER_REQUEST_READ + BROKER_REQUEST_DEADLINE).await;
        let reply = r#"{"ok":true,"value":{"key":"REDACTED","expiresAt":"2030-05-05T13:00:00.000Z","keyId":7}}"#;
        stream
            .get_mut()
            .write_all(format!("{reply}\n").as_bytes())
            .await
            .unwrap();
    });

    let answer = Broker::new(path).personal_key().await;

    assert!(answer.is_ok(), "{:?}", answer.err());
}

/// The broker's limits as its own files set them, so a change there fails here first.
fn broker_file(name: &str) -> String {
    let path = format!(
        "{}/../../deploy/headscale/broker/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

/// The number in `text` right after `prefix`, ignoring `_` separators.
fn number_after(text: &str, prefix: &str) -> u64 {
    let (_, rest) = text
        .split_once(prefix)
        .unwrap_or_else(|| panic!("no {prefix:?}"));
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .filter(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap()
}

#[test]
fn bd_10_the_call_limit_outlasts_the_broker_s_worst_case_as_its_files_set_it() {
    let unit = broker_file("anima-broker@.unit");
    let main = broker_file("main.ts");
    assert_eq!(number_after(&unit, "flock -w "), BROKER_LOCK_WAIT.as_secs());
    assert_eq!(
        number_after(&unit, "RuntimeMaxSec="),
        BROKER_RUNTIME_MAX.as_secs()
    );
    let read_ms = u128::from(number_after(&main, "const READ_MS = "));
    assert_eq!(read_ms, BROKER_REQUEST_READ.as_millis());
    let request_ms = u128::from(number_after(&main, "const REQUEST_MS = "));
    assert_eq!(request_ms, BROKER_REQUEST_DEADLINE.as_millis());

    assert!(BROKER_LOCK_WAIT + BROKER_REQUEST_READ + BROKER_REQUEST_DEADLINE <= BROKER_RUNTIME_MAX);
    assert!(
        CALL_LIMIT > BROKER_RUNTIME_MAX,
        "the hub gives up only after the broker is gone"
    );
}

/// The reader's node read holds the turn in a task of its own: the read publishes after
/// `BROKER_WAIT` without the list, and the turn stays taken until the call ends.
#[tokio::test(start_paused = true)]
async fn bd_15_a_hung_node_read_holds_up_the_reader_only_for_its_broker_wait() {
    let folder = tempfile::tempdir().unwrap();
    let (broker, accepted) = hung_broker(&folder);

    let started = Instant::now();
    let read = read_broker(Some(&broker), BROKER_WAIT).await;

    assert!(matches!(read, Some(Err(Error::BrokerBusy))), "{read:?}");
    assert_eq!(started.elapsed(), BROKER_WAIT);
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    let again = broker.nodes_if_free().await;
    assert!(
        matches!(again, Err(Error::BrokerBusy)),
        "the turn is still held"
    );
    tokio::time::sleep(CALL_LIMIT).await;
    assert!(matches!(
        broker.nodes_if_free().await,
        Err(Error::BrokerTimedOut)
    ));
}
