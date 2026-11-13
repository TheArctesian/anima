//! The checks and maintenance parts of the hub schema, with every loader they read, answer
//! through `build_schema` (API-005, CHK-005, MW-001).

use std::sync::atomic::Ordering;

use anima_hub::{fleet::sessions::Live, schema::build_schema};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{PVE_01, add_operator, data, hub_schema, wired};
use crate::devices::{batching::counting_pool, graphql_read::add_bulk};

/// pve-01's seeded ICMP check.
const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";

/// Every field a checks loader serves, on one check.
const CHECK_FIELDS: &str = "name status lastHeartbeat { at } uptime(window: DAY) \
    heartbeats(first: 5) { edges { node { status } } pageInfo { hasNextPage } }";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks", "maintenance"))
)]
async fn h_4_a_check_and_the_maintenance_windows_answer_through_the_hub_schema(pool: PgPool) {
    let schema = hub_schema(&pool, &Live::default());
    let result = data(
        &schema,
        &format!("{{ check(id: \"{PVE_01_PING}\") {{ {CHECK_FIELDS} }} maintenanceWindows {{ id name }} }}"),
        json!({}),
    )
    .await;
    let check = &result["check"];
    assert_eq!(check["name"], "Ping");
    assert_eq!(check["status"], "UNKNOWN", "no beat yet");
    assert_eq!(check["lastHeartbeat"], Value::Null);
    assert_eq!(check["heartbeats"]["edges"], json!([]));
    let stored: i64 = sqlx::query_scalar("select count(*) from maintenance.windows")
        .fetch_one(&pool)
        .await
        .unwrap();
    let shown = result["maintenanceWindows"].as_array().unwrap().len();
    assert_eq!(i64::try_from(shown).unwrap(), stored);
    assert!(stored > 0, "the fixture seeds windows");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn h_4_devices_show_their_status_uptime_ip_and_checks_through_the_hub_schema(pool: PgPool) {
    let schema = hub_schema(&pool, &Live::default());
    let result = data(
        &schema,
        &format!(
            "{{ devices(first: 50, sort: STATUS) {{ totalCount edges {{ node {{ id status \
             uptime(window: DAY) ip checks {{ {CHECK_FIELDS} }} }} }} }} }}"
        ),
        json!({}),
    )
    .await;
    assert_eq!(result["devices"]["totalCount"], 13);
    let pve = result["devices"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| &edge["node"])
        .find(|node| node["id"] == PVE_01)
        .unwrap();
    let names: Vec<&str> = pve["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["name"].as_str().unwrap())
        .collect();
    let stored: Vec<String> = sqlx::query_scalar(
        "select name from checks.checks where device_id = $1::text::uuid order by name",
    )
    .bind(PVE_01)
    .fetch_all(&pool)
    .await
    .unwrap();
    let mut names = names;
    names.sort_unstable();
    assert_eq!(names, stored);
    assert_eq!(pve["status"], "UNKNOWN");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn h_4_check_and_window_mutations_run_through_the_hub_schema(pool: PgPool) {
    add_operator(&pool).await;
    let schema = hub_schema(&pool, &Live::default());
    let created = data(
        &schema,
        "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id name paused } }",
        json!({ "input": { "deviceId": PVE_01, "name": "Web", "config": { "tcp": { "port": 443 } } } }),
    )
    .await;
    let id = created["createCheck"]["id"].as_str().unwrap();
    let paused = data(
        &schema,
        "mutation($id: ID!) { setCheckPaused(id: $id, paused: true) { paused } }",
        json!({ "id": id }),
    )
    .await;
    let window = data(
        &schema,
        r#"mutation { createMaintenanceWindow(input: { name: "Patch", kind: MANUAL, timezone: "UTC", targets: [{ type: DEVICE, target: "0190a000-0000-7000-8002-000000000001" }] }) { id name } }"#,
        json!({}),
    )
    .await;

    assert_eq!(created["createCheck"]["name"], "Web");
    assert_eq!(paused["setCheckPaused"]["paused"], true);
    assert_eq!(window["createMaintenanceWindow"]["name"], "Patch");
}

#[tokio::test]
async fn h_4_the_hub_schema_declares_the_checks_subscriptions() {
    let sdl = crate::hub::hub_sdl();
    let subscription = sdl
        .split("type Subscription {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .unwrap_or_else(|| panic!("the hub declares a subscription root:\n{sdl}"));
    for field in ["deviceStatusChanged(", "heartbeatAdded(", "eventAdded"] {
        assert!(subscription.contains(field), "{field} in {subscription}");
    }
}

/// Every devices and checks loader a devices page reads, in one selection.
const DEVICE_PAGE: &str = "{ devices(first: 200) { edges { node { status uptime(window: DAY) \
    ip checks { status lastHeartbeat { at } uptime(window: DAY) \
    heartbeats(first: 50) { edges { node { status } } } } } } } }";

/// Batched, a page costs about 15 statements whatever its size. A debug build can still split
/// batches on the large set (29 to 67 statements were seen with the hub's 10 ms window, more on
/// a loaded machine), so the bounds leave room for that. Loading per key costs at least one
/// statement per check field, over 1300 on 163 devices.
const FEW_AT_MOST: usize = 30;
const MANY_AT_MOST: usize = 200;

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn api_005_the_hub_schema_batches_the_device_and_check_loaders(pool: PgPool) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = build_schema(wired(&counted, &Live::default()));

    let few = data(&schema, DEVICE_PAGE, json!({})).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, 30, 150).await;
    let many = data(&schema, DEVICE_PAGE, json!({})).await;
    let many_statements = statements.load(Ordering::SeqCst);

    let shown = |page: &Value| page["devices"]["edges"].as_array().unwrap().len();
    assert_eq!((shown(&few), shown(&many)), (13, 163));
    assert!(
        few_statements <= FEW_AT_MOST && many_statements <= MANY_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}
