//! The hub's broker client over a real Unix socket, through the reader and the `vpn` query: the
//! merge of the node list, the broker states and the turn (R-055, VP-7, BD-10, BD-15, BD-16).
//! The fake broker accepts one connection at a time and closes a second at once, as systemd's
//! `MaxConnections=1` does.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use anima_hub::vpn::{Broker, Feed, Reading, spawn_reader};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{UnixListener, UnixStream},
};

use super::{
    graphql::{STATUS, create_site, data, hub_reading, published, reading},
    proxy::{self, Reply},
};

/// A fake broker on a socket in its own temporary folder.
pub struct FakeBroker {
    folder: tempfile::TempDir,
    seen: Arc<Mutex<Vec<String>>>,
    refused: Arc<AtomicUsize>,
}

impl FakeBroker {
    pub fn path(&self) -> PathBuf {
        self.folder.path().join("broker.sock")
    }

    pub fn broker(&self) -> Broker {
        Broker::new(self.path())
    }

    /// The request lines seen so far.
    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }

    /// Connections closed at once because another one was open.
    pub fn refused(&self) -> usize {
        self.refused.load(Ordering::SeqCst)
    }
}

/// A broker reply with `value`.
pub fn ok(value: &Value) -> String {
    json!({ "ok": true, "value": value }).to_string()
}

/// A broker refusal line.
pub fn refusal(word: &str, detail: &str) -> String {
    json!({ "ok": false, "refusal": word, "detail": detail }).to_string()
}

/// A broker that answers each request line with `answer(line)` after `delay`; `None` closes
/// the connection with no reply.
pub fn start(
    delay: Duration,
    answer: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
) -> FakeBroker {
    let fake = FakeBroker {
        folder: tempfile::tempdir().unwrap(),
        seen: Arc::default(),
        refused: Arc::default(),
    };
    let listener = UnixListener::bind(fake.path()).unwrap();
    let (seen, refused) = (fake.seen.clone(), fake.refused.clone());
    let answer = Arc::new(answer);
    let busy = Arc::new(AtomicBool::new(false));
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            if busy.swap(true, Ordering::SeqCst) {
                refused.fetch_add(1, Ordering::SeqCst);
                continue;
            }
            let (answer, seen, busy) = (answer.clone(), seen.clone(), busy.clone());
            tokio::spawn(async move {
                serve_one(stream, delay, |line| {
                    seen.lock().unwrap().push(line.to_owned());
                    answer(line)
                })
                .await;
                busy.store(false, Ordering::SeqCst);
            });
        }
    });
    fake
}

async fn serve_one(stream: UnixStream, delay: Duration, answer: impl Fn(&str) -> Option<String>) {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    if stream.read_line(&mut line).await.is_err() {
        return;
    }
    tokio::time::sleep(delay).await;
    if let Some(reply) = answer(line.trim_end()) {
        let _ = stream
            .get_mut()
            .write_all(format!("{reply}\n").as_bytes())
            .await;
    }
    let _ = stream.get_mut().shutdown().await;
}

/// A broker node as `broker/nodes.ts` writes it.
pub fn broker_node(
    id: u64,
    name: &str,
    (ip, tags): (&str, &[&str]),
    routes: [&[&str]; 2],
) -> Value {
    let [approved, available] = routes;
    let user = if tags.is_empty() {
        "operator"
    } else {
        "tagged-devices"
    };
    json!({ "id": id, "name": name, "user": user, "tags": tags, "ips": [ip], "online": true,
            "expiry": null, "approved": approved, "available": available })
}

/// The `nodes` value: `router-home` with a second route waiting, `router-office` waiting for
/// office1's subnet, and the operator's `mac`.
pub fn nodes_value() -> Value {
    json!({
        "nodes": [
            broker_node(2, "mac", ("100.64.0.2", &[]), [&[], &[]]),
            broker_node(3, "router-home", ("100.64.0.3", &["tag:router", "tag:site-home"]),
                [&["10.10.0.0/24"], &["10.10.0.0/24", "10.30.0.0/24"]]),
            broker_node(9, "router-office", ("100.64.0.9", &["tag:router", "tag:site-office1"]),
                [&[], &["10.20.0.0/24"]]),
        ],
        "sites": [{ "name": "home", "subnet": "10.10.0.0/24" },
                  { "name": "office1", "subnet": "10.20.0.0/24" }],
    })
}

/// A reader of `STATUS` that also calls `broker`, publishing on `feed`.
pub async fn read_with(pool: &PgPool, feed: &Feed, broker: Broker) -> tokio::task::JoinHandle<()> {
    let proxy = proxy::start(|| Reply::json(200, STATUS)).await;
    let reading = Reading {
        broker: Some(broker),
        ..reading(pool, feed)
    };
    let reader = spawn_reader(Some(proxy.address()), reading).unwrap();
    published(feed, |_| true).await;
    reader
}

const BROKER: &str = "{ vpn { broker nodes { name waitingRoutes } \
    sites { site { name } waitingRouter { name address role path keyState waitingRoutes inAnima } } } }";

/// The `vpn` picture after one read through `broker`.
async fn shown_with(pool: &PgPool, broker: Broker) -> Value {
    let feed = Feed::new();
    let (hub, token) = hub_reading(pool, &feed, Some(broker.clone())).await;
    create_site(&hub, &token, "home", "10.10.0.0/24").await;
    create_site(&hub, &token, "office1", "10.20.0.0/24").await;
    let reader = read_with(pool, &feed, broker).await;
    let shown = data(&hub, &token, BROKER, json!({})).await;
    reader.abort();
    shown["vpn"].clone()
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_nodes_reply_is_merged_as_waiting_routes(pool: PgPool) {
    let fake = start(Duration::ZERO, |_| Some(ok(&nodes_value())));

    let shown = shown_with(&pool, fake.broker()).await;

    assert_eq!(fake.seen(), [r#"{"verb":"nodes"}"#]);
    assert_eq!(shown["broker"], "READY");
    assert_eq!(
        shown["nodes"],
        json!([{ "name": "mac", "waitingRoutes": [] },
               { "name": "router-home", "waitingRoutes": ["10.30.0.0/24"] }])
    );
    assert_eq!(shown["sites"][0]["waitingRouter"], Value::Null);
    assert_eq!(
        shown["sites"][1]["waitingRouter"],
        json!({ "name": "router-office", "address": "100.64.0.9", "role": "ROUTER",
                "path": "UNKNOWN", "keyState": "NEVER", "waitingRoutes": ["10.20.0.0/24"],
                "inAnima": false })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_missing_socket_is_not_installed(pool: PgPool) {
    let folder = tempfile::tempdir().unwrap();
    let broker = Broker::new(folder.path().join("broker.sock"));

    let shown = shown_with(&pool, broker).await;

    assert_eq!(shown["broker"], "NOT_INSTALLED");
    assert_eq!(shown["sites"][1]["waitingRouter"], Value::Null);
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_eof_with_no_reply_is_unreadable(pool: PgPool) {
    let fake = start(Duration::ZERO, |_| None);

    let shown = shown_with(&pool, fake.broker()).await;

    assert_eq!(fake.seen().len(), 1);
    assert_eq!(shown["broker"], "FAILED");
    assert_eq!(shown["nodes"][1]["waitingRoutes"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_reply_over_64_kib_is_refused(pool: PgPool) {
    // A valid reply whose one line is longer than 64 KiB.
    let mut value = nodes_value();
    value["nodes"][0]["name"] = "m".repeat(64 * 1024).into();
    let fake = start(Duration::ZERO, move |_| Some(ok(&value)));

    let shown = shown_with(&pool, fake.broker()).await;

    assert_eq!(shown["broker"], "FAILED");
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_mutation_during_a_read_takes_its_turn(pool: PgPool) {
    let fake = start(Duration::from_millis(300), |line| {
        let request: Value = serde_json::from_str(line).unwrap();
        Some(if request["verb"] == "nodes" {
            ok(&nodes_value())
        } else {
            ok(&json!({ "key": "REDACTED", "expiresAt": "2030-05-05T13:00:00.000Z", "keyId": 7 }))
        })
    });
    let feed = Feed::new();
    let broker = fake.broker();
    let (hub, token) = hub_reading(&pool, &feed, Some(broker.clone())).await;
    let invite = "mutation { createVpnInvite { expiresAt } }";

    // The reader holds the shared turn for 300 ms; without it the mutation's connection would
    // be closed while the reader waits for its reply.
    let (reader, made) = tokio::join!(read_with(&pool, &feed, broker), async {
        while fake.seen().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        data(&hub, &token, invite, json!({})).await
    });
    reader.abort();

    assert_eq!(
        made["createVpnInvite"]["expiresAt"],
        "2030-05-05T13:00:00+00:00"
    );
    assert_eq!(fake.refused(), 0);
    assert_eq!(fake.seen().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn bd_15_a_read_while_another_call_holds_the_turn_does_not_connect(pool: PgPool) {
    let fake = start(Duration::from_millis(300), |_| Some(ok(&nodes_value())));
    let broker = fake.broker();
    let (first, second) = (Feed::new(), Feed::new());

    // Two readers on clones of one broker read at once: the second finds the turn taken,
    // publishes without the broker's list and never connects.
    let (one, two) = tokio::join!(
        read_with(&pool, &first, broker.clone()),
        read_with(&pool, &second, broker)
    );
    one.abort();
    two.abort();

    assert_eq!(fake.refused(), 0);
    assert_eq!(fake.seen().len(), 1);
    let mut states = [first, second].map(|feed| format!("{:?}", feed.latest().unwrap().broker));
    states.sort();
    assert_eq!(
        states,
        ["Failed", "Ready"],
        "a busy first read has no last state to keep"
    );
}
