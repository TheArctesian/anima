//! The VPN keys through a fake broker on a Unix socket: an invite and a router key the broker
//! makes, an auth ID registered, a key renewed, and a broker reply the hub cannot read that
//! holds a key; with a fake `LocalAPI` proxy so the hub reads the broker's node list first
//! (VPN-001, VPN-003, VP-7, VP-10, BD-9, LOG-007).

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anima_hub::{
    clock::FixedClock,
    events::Bus,
    vpn::{Broker, Feed, Reading, spawn_reader},
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader},
    net::{TcpListener, UnixListener},
};

use super::{
    Secrets,
    edge::{WholeHub, code},
};

/// The hub's status as a `LocalAPI` proxy answers it.
const STATUS: &str = include_str!("../../../../src/vpn/tests/fixtures/status-active.json");
/// `home` in the `devices` fixture, whose subnet the broker's policy names.
const HOME: &str = "0190a000-0000-7000-8001-000000000001";

/// A broker that answers each verb, its keys canaries; it lives as long as its folder.
pub struct FakeBroker {
    folder: tempfile::TempDir,
    keys: [String; 3],
    auth_id: String,
}

/// The `nodes` value: the operator's untagged `mac`, and `home` in the policy.
fn nodes() -> Value {
    json!({
        "nodes": [{ "id": 2, "name": "mac", "user": "operator", "tags": [], "ips": ["100.64.0.2"],
                    "online": true, "expiry": null, "approved": [], "available": [] }],
        "sites": [{ "name": "home", "subnet": "10.10.0.0/24" }],
    })
}

/// The broker's reply to `request`: a key the first time an invite is asked, a value the hub
/// cannot read, holding a key, the second time.
fn answer(request: &Value, keys: &[String; 3], invites: &AtomicUsize) -> Value {
    let expiry = "2030-05-05T13:00:00.000Z";
    let node = |name| json!({ "name": name, "address": "100.64.0.2", "keyExpiry": expiry });
    let value = match request["verb"].as_str().unwrap() {
        "nodes" => nodes(),
        "personal-key" if invites.fetch_add(1, Ordering::SeqCst) > 0 => json!(keys[2]),
        "personal-key" => json!({ "key": keys[0], "expiresAt": expiry, "keyId": 7 }),
        "router-key" => json!({ "key": keys[1], "expiresAt": expiry, "keyId": 8 }),
        "register" => node("laptop"),
        _ => node("mac"),
    };
    json!({ "ok": true, "value": value })
}

impl FakeBroker {
    /// Starts the broker, planting its keys and the auth ID it is sent.
    pub fn start(secrets: &mut Secrets) -> Self {
        let keys = ["vpn-invite-key", "vpn-router-key", "vpn-unreadable-reply"]
            .map(|kind| secrets.plant(kind));
        let auth_id =
            secrets.plant_with("vpn-auth-id", |hex| format!("hskey-authreq-{}", &hex[..24]));
        let fake = Self {
            folder: tempfile::tempdir().unwrap(),
            keys: keys.clone(),
            auth_id,
        };
        let listener = UnixListener::bind(fake.path()).unwrap();
        let invites = Arc::new(AtomicUsize::new(0));
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                if stream.read_line(&mut line).await.is_ok() {
                    let request: Value = serde_json::from_str(&line).unwrap();
                    let reply = format!("{}\n", answer(&request, &keys, &invites));
                    let _ = stream.get_mut().write_all(reply.as_bytes()).await;
                }
                let _ = stream.get_mut().shutdown().await;
            }
        });
        fake
    }

    fn path(&self) -> PathBuf {
        self.folder.path().join("broker.sock")
    }

    pub fn broker(&self) -> Broker {
        Broker::new(self.path())
    }
}

/// A `LocalAPI` proxy on 127.0.0.1 that answers each request with [`STATUS`].
async fn proxy() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read_exact(&mut byte).await.is_ok() {
                head.push(byte[0]);
            }
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{STATUS}",
                STATUS.len()
            );
            let _ = stream.write_all(reply.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });
    address
}

/// Reads the VPN once into `feed` through `fake`, then makes each VPN change with the
/// operator's `token`.
pub async fn change(
    hub: &WholeHub,
    token: &str,
    (pool, feed, fake): (&PgPool, &Feed, &FakeBroker),
    now: DateTime<Utc>,
) {
    let reading = Reading {
        pool: pool.clone(),
        bus: Bus::new(16),
        clock: Arc::new(FixedClock(now)),
        feed: feed.clone(),
        broker: Some(fake.broker()),
    };
    let reader = spawn_reader(Some(proxy().await), reading).unwrap();
    let mut pictures = feed.subscribe();
    let read = pictures.wait_for(Option::is_some);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), read)
            .await
            .is_ok(),
        "a picture"
    );

    let invite = "mutation { createVpnInvite { key } }";
    let made = hub.data(token, invite, json!({})).await;
    assert_eq!(
        made["createVpnInvite"]["key"], fake.keys[0],
        "the key is shown once"
    );
    let router = "mutation($s: ID!) { createRouterKey(siteId: $s) { key } }";
    let made = hub.data(token, router, json!({ "s": HOME })).await;
    assert_eq!(
        made["createRouterKey"]["key"], fake.keys[1],
        "the key is shown once"
    );
    let register = "mutation($a: String!) { registerVpnNode(authId: $a) { name } }";
    let registered = hub
        .data(token, register, json!({ "a": fake.auth_id }))
        .await;
    assert_eq!(registered["registerVpnNode"]["name"], "laptop");
    let renew = "mutation($n: String!) { renewVpnKey(name: $n) { name } }";
    let renewed = hub.data(token, renew, json!({ "n": "mac" })).await;
    assert_eq!(renewed["renewVpnKey"]["name"], "mac");
    let unreadable = hub.as_bearer(token, invite, json!({})).await;
    assert_eq!(code(&unreadable), "UNAVAILABLE", "an unreadable reply");
    reader.abort();
}
