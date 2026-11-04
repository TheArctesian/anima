//! `Check.status`, `lastHeartbeat` and `uptime` cost a bounded number of statements however
//! many checks a query selects (API-005).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::status::Status;
use serde_json::{Value, json};
use sqlx::PgPool;
use tracing::{Event, Subscriber, subscriber::set_default};
use tracing_subscriber::{Layer, layer::Context, layer::SubscriberExt as _, registry::Registry};

use super::harness::{
    TestSchema, beat, create, data_with, database_now, input, record, schema_at, signed_in,
};

/// Counts the statements sqlx runs on this thread.
struct Statements(Arc<AtomicUsize>);

impl<S: Subscriber> Layer<S> for Statements {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() == "sqlx::query" {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

const DEVICES: [&str; 5] = [
    "0190a000-0000-7000-8002-000000000001",
    "0190a000-0000-7000-8002-000000000002",
    "0190a000-0000-7000-8002-000000000003",
    "0190a000-0000-7000-8002-000000000004",
    "0190a000-0000-7000-8002-000000000005",
];

const FIELDS: &str = "status lastHeartbeat { status } uptime(window: DAY)";

/// The statements the query needs, less the two each group takes to be read and to list its
/// children.
async fn loader_statements(schema: &TestSchema, groups: &[String], selection: &str) -> usize {
    let fields: Vec<String> = groups
        .iter()
        .enumerate()
        .map(|(index, id)| {
            format!(
                "g{index}: check(id: \"{id}\") {{ config {{ ... on GroupConfig {{ children {{ \
                 {selection} }} }} }} }}"
            )
        })
        .collect();
    let query = format!("query {{ {} }}", fields.join(" "));
    let count = Arc::new(AtomicUsize::new(0));
    let guard = set_default(Registry::default().with(Statements(count.clone())));
    tracing::callsite::rebuild_interest_cache();
    let response = schema.execute(signed_in(&query)).await;
    drop(guard);
    assert_eq!(response.errors.len(), 0, "{:?}", response.errors);
    let data: Value = response.data.into_json().unwrap();
    assert_eq!(data.as_object().unwrap().len(), groups.len());
    count
        .load(Ordering::SeqCst)
        .saturating_sub(2 * groups.len())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_status_last_heartbeat_and_uptime_cost_a_bounded_number_of_statements(
    pool: PgPool,
) {
    let at = database_now(&pool).await;
    let schema = schema_at(&pool, at).0;
    let mut groups = Vec::new();
    for (device_index, device) in DEVICES.iter().enumerate() {
        let mut children = Vec::new();
        for port in 0..4 {
            let name = format!("port-{device_index}-{port}");
            let tcp = json!({ "tcp": { "port": 22 + port } });
            let id = create(&schema, input(device, &name, tcp)).await;
            record(&pool, &id, &beat(at, Status::Up, Some(3))).await;
            children.push(id);
        }
        let config = json!({ "group": { "childIds": children } });
        groups.push(create(&schema, input(device, "group", config)).await);
    }
    let one = loader_statements(&schema, &groups[..1], FIELDS).await;
    let many = loader_statements(&schema, &groups, FIELDS).await;
    // Per check the three fields would take about three statements each: 12 for 4 checks and
    // 60 for 20. Batched, the cost is a few statements per batch, whatever the number of checks.
    assert!(one <= 8, "4 checks took {one} statements");
    assert!(many <= 20, "20 checks on 5 devices took {many} statements");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_the_loaders_agree_with_the_per_check_rules(pool: PgPool) {
    let at = database_now(&pool).await;
    let schema = schema_at(&pool, at).0;
    let up = create(
        &schema,
        input(DEVICES[0], "up", json!({ "tcp": { "port": 22 } })),
    )
    .await;
    let down = create(
        &schema,
        input(DEVICES[1], "down", json!({ "tcp": { "port": 22 } })),
    )
    .await;
    record(&pool, &up, &beat(at, Status::Up, Some(3))).await;
    record(&pool, &down, &beat(at, Status::Down, None)).await;
    let query = "query($a: ID!, $b: ID!) { a: check(id: $a) { status lastHeartbeat { status } uptime(window: DAY) } \
        b: check(id: $b) { status lastHeartbeat { status } uptime(window: DAY) } }";
    let read = data_with(&schema, query, json!({ "a": up, "b": down })).await;
    assert_eq!(
        read["a"],
        json!({ "status": "UP", "lastHeartbeat": { "status": "UP" }, "uptime": 1.0 })
    );
    assert_eq!(
        read["b"],
        json!({ "status": "DOWN", "lastHeartbeat": { "status": "DOWN" }, "uptime": 0.0 })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_heartbeat_pages_of_many_checks_on_many_devices_take_a_bounded_number_of_statements(
    pool: PgPool,
) {
    let at = database_now(&pool).await;
    let schema = schema_at(&pool, at).0;
    let devices: Vec<String> =
        sqlx::query_scalar("SELECT id::text FROM devices.devices ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(devices.len(), 13);
    let mut groups = Vec::new();
    for (device_index, device) in devices.iter().enumerate() {
        let mut children = Vec::new();
        for port in 0..2 {
            let name = format!("port-{device_index}-{port}");
            let tcp = json!({ "tcp": { "port": 22 + port } });
            let id = create(&schema, input(device, &name, tcp)).await;
            for seconds in (0..3).rev() {
                let when = at - chrono::TimeDelta::seconds(seconds);
                record(&pool, &id, &beat(when, Status::Up, Some(3))).await;
            }
            children.push(id);
        }
        let config = json!({ "group": { "childIds": children } });
        groups.push(create(&schema, input(device, "group", config)).await);
    }
    let pages = "heartbeats(first: 50) { edges { node { status } } pageInfo { hasNextPage } }";
    let many = loader_statements(&schema, &groups, pages).await;
    // One statement per batch, not one per check: the 39 checks would take 39 by themselves.
    // The bound leaves room for a batch the loader splits on a busy machine.
    assert!(
        many < devices.len(),
        "39 checks on 13 devices took {many} statements"
    );
}
