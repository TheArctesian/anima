//! The VPN picture through the hub: the `vpn` query, the `vpnChanged` subscription, and what
//! reaches the process log (VPN-005, VPN-008, VPN-009, VPN-011, LOG-007, K1).

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Bus, Operator, process_log},
        vpn::{Broker, Feed, Picture, Reading, spawn_reader},
    },
    ids::OperatorId,
    schema::{Dependencies, build_schema},
};
use async_graphql::{Request, futures_util::StreamExt as _};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::proxy::{self, Reply};
use crate::{
    hub::{Hub, dependencies, set_operator, start},
    logs::Capture,
};

/// The hub's status as the fake proxy answers it: three nodes, `hub`, `mac`, `router-home`.
pub const STATUS: &str = include_str!("../../../src/fleet/vpn/tests/fixtures/status-active.json");

pub fn reading(pool: &PgPool, feed: &Feed) -> Reading {
    Reading {
        pool: pool.clone(),
        bus: Bus::new(16),
        clock: Arc::new(FixedClock(start())),
        feed: feed.clone(),
        broker: None,
    }
}

/// Waits until the feed holds a picture that `accepts` accepts.
pub async fn published(feed: &Feed, accepts: impl Fn(&Picture) -> bool) {
    let mut receiver = feed.subscribe();
    let waited = tokio::time::timeout(
        Duration::from_secs(5),
        receiver.wait_for(|latest| latest.as_deref().is_some_and(&accepts)),
    );
    assert!(waited.await.is_ok(), "no picture in time");
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_no_log_line_holds_status_text(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("info", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    // The failed answer holds the status text too, so a logged body would show.
    let proxy = proxy::start(move || match counted.fetch_add(1, Ordering::SeqCst) {
        0 => Reply::json(200, STATUS),
        _ => Reply::json(500, STATUS),
    })
    .await;
    let feed = Feed::new();

    let first = spawn_reader(Some(proxy.address()), reading(&pool, &feed)).unwrap();
    published(&feed, |picture| picture.fresh).await;
    let second = spawn_reader(Some(proxy.address()), reading(&pool, &feed)).unwrap();
    published(&feed, |picture| !picture.fresh).await;
    first.abort();
    second.abort();

    let asked = proxy.seen();
    assert!(
        asked
            .iter()
            .all(|head| head.starts_with("GET /localapi/v0/status?peers=true ")),
        "{asked:?}"
    );
    let text = capture.text();
    assert!(text.contains("VPN status read failed"), "{text}");
    for leak in [
        "router-home",
        "HostName",
        "nodekey:",
        "100.64.",
        "192.0.2.",
        "tag:",
    ] {
        assert!(!text.contains(leak), "{leak:?} in {text}");
    }
}

const PICTURE: &str = "{ vpn { readAt fresh hub { name role inAnima } \
    sites { site { name } router { name } state path relayRegion hasPingCheck } \
    nodes { name address os role tags routes online keyState path inAnima device { id name } } } }";

/// A signed-in hub whose `vpn` reads `feed` and whose VPN mutations call `broker`.
pub async fn hub_reading(pool: &PgPool, feed: &Feed, broker: Option<Broker>) -> (Hub, String) {
    set_operator(pool).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let parts = Dependencies {
        vpn_feed: feed.clone(),
        vpn_broker: broker,
        ..dependencies(pool, clock, Bus::new(16))
    };
    let hub = Hub::serving(pool, parts);
    let token = hub.token().await;
    (hub, token)
}

/// Starts a reader on `STATUS` and waits for its first picture.
async fn read_status(pool: &PgPool, feed: &Feed) -> tokio::task::JoinHandle<()> {
    let proxy = proxy::start(|| Reply::json(200, STATUS)).await;
    let reader = spawn_reader(Some(proxy.address()), reading(pool, feed)).unwrap();
    published(feed, |_| true).await;
    reader
}

/// Runs `query` and returns its data; panics on errors.
pub async fn data(hub: &Hub, token: &str, query: &str, variables: Value) -> Value {
    let response = hub.post(Some(token), query, variables).await;
    assert!(response["errors"].is_null(), "{response}");
    response["data"].clone()
}

pub async fn create_site(hub: &Hub, token: &str, name: &str, subnet: &str) -> String {
    let query = "mutation($n: String!, $s: Cidr!) { createSite(name: $n, subnet: $s) { id } }";
    let made = data(hub, token, query, json!({ "n": name, "s": subnet })).await;
    made["createSite"]["id"].as_str().unwrap().to_owned()
}

/// Creates the device `(name, host)` in `site`; returns its id.
async fn create_device(hub: &Hub, token: &str, site: &str, (name, host): (&str, &str)) -> String {
    let query = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { id } }";
    let input = json!({ "siteId": site, "name": name, "host": host, "addDefaultCheck": false });
    let made = data(hub, token, query, json!({ "input": input })).await;
    made["createDevice"]["id"].as_str().unwrap().to_owned()
}

async fn create_ping_check(hub: &Hub, token: &str, device: &str) {
    let query = "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id } }";
    let input =
        json!({ "deviceId": device, "name": "VPN path", "config": { "tailscalePing": {} } });
    data(hub, token, query, json!({ "input": input })).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_005_vpn_is_null_without_a_proxy(pool: PgPool) {
    let feed = Feed::new();
    let (hub, token) = hub_reading(&pool, &feed, None).await;

    assert!(spawn_reader(None, reading(&pool, &feed)).is_none());

    assert_eq!(
        data(&hub, &token, "{ vpn { fresh } }", json!({})).await,
        json!({ "vpn": null })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_009_vpn_returns_each_site_with_router_state_and_path(pool: PgPool) {
    let feed = Feed::new();
    let (hub, token) = hub_reading(&pool, &feed, None).await;
    let home = create_site(&hub, &token, "home", "10.10.0.0/24").await;
    create_site(&hub, &token, "office1", "10.20.0.0/24").await;
    let router = create_device(&hub, &token, &home, ("router-home", "100.64.0.3")).await;
    create_ping_check(&hub, &token, &router).await;
    let reader = read_status(&pool, &feed).await;

    let shown = data(&hub, &token, PICTURE, json!({})).await;
    reader.abort();

    assert_eq!(
        shown["vpn"]["sites"],
        json!([
            { "site": { "name": "home" }, "router": { "name": "router-home" }, "state": "SERVING",
              "path": "DIRECT", "relayRegion": null, "hasPingCheck": true },
            { "site": { "name": "office1" }, "router": null, "state": "NO_ROUTER",
              "path": "UNKNOWN", "relayRegion": null, "hasPingCheck": false },
        ])
    );
    assert_eq!(shown["vpn"]["fresh"], true);
    assert_eq!(shown["vpn"]["readAt"], start().to_rfc3339());
    assert_eq!(
        shown["vpn"]["hub"],
        json!({ "name": "hub", "role": "HUB", "inAnima": true })
    );
    let routes: Vec<&Value> = shown["vpn"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| &node["routes"])
        .collect();
    assert_eq!(routes, [&json!([]), &json!(["10.10.0.0/24"])]);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_011_a_peer_with_a_device_resolves_device(pool: PgPool) {
    let feed = Feed::new();
    let (hub, token) = hub_reading(&pool, &feed, None).await;
    let home = create_site(&hub, &token, "home", "10.10.0.0/24").await;
    let mac = create_device(&hub, &token, &home, ("operator-mac", "100.64.0.2")).await;
    let reader = read_status(&pool, &feed).await;

    let shown = data(&hub, &token, PICTURE, json!({})).await;
    reader.abort();

    let nodes = &shown["vpn"]["nodes"];
    assert_eq!(nodes[0]["name"], "mac");
    assert_eq!(nodes[0]["address"], "100.64.0.2");
    assert_eq!(
        nodes[0]["device"],
        json!({ "id": mac, "name": "operator-mac" })
    );
    assert_eq!(nodes[0]["inAnima"], true);
    assert_eq!(nodes[0]["keyState"], "EXPIRING");
    assert_eq!(nodes[1]["device"], Value::Null);
    assert_eq!(nodes[1]["tags"], json!(["tag:router", "tag:site-home"]));
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_005_vpn_changed_sends_the_next_picture(pool: PgPool) {
    let feed = Feed::new();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        vpn_feed: feed.clone(),
        ..dependencies(&pool, clock, Bus::new(16))
    });
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: anima_hub::fleet::events::Actor::for_tests(),
    };
    let request = Identity::for_tests(operator, start() + TimeDelta::days(3650)).attach(
        Request::new("subscription { vpnChanged { fresh nodes { name path } } }"),
    );
    let mut stream = schema.execute_stream(request);
    // Polled once, the stream subscribes to the feed; nothing is published yet.
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "nothing was published yet");

    let reader = read_status(&pool, &feed).await;
    let received = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    reader.abort();

    assert!(received.errors.is_empty(), "{:?}", received.errors);
    let picture = received.data.into_json().unwrap();
    assert_eq!(picture["vpnChanged"]["fresh"], true);
    assert_eq!(
        picture["vpnChanged"]["nodes"],
        json!([{ "name": "mac", "path": "UNKNOWN" }, { "name": "router-home", "path": "DIRECT" }])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_005_an_api_key_may_read_vpn(pool: PgPool) {
    let feed = Feed::new();
    let (hub, token) = hub_reading(&pool, &feed, None).await;
    let reader = read_status(&pool, &feed).await;
    let create = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "name": "status", "scope": "READ", "madeFor": "USER", "expiresAt": start() + TimeDelta::days(30) });
    let made = data(&hub, &token, create, json!({ "input": input })).await;
    let secret = made["createApiKey"]["secret"].as_str().unwrap();

    let shown = data(&hub, secret, "{ vpn { fresh nodes { name } } }", json!({})).await;
    reader.abort();

    assert_eq!(shown["vpn"]["nodes"].as_array().unwrap().len(), 2);
}
