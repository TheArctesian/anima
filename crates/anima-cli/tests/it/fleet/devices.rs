//! `anima device list` against the scripted hub, and the fixtures of the device tests (CMD-020,
//! CMD-026).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

mod batteries;
mod get;

const TOKEN: &str = "login-token-for-device-tests";

/// The K6 Phase 1 devices as `DeviceList` nodes, with their site names.
pub fn fixture_devices() -> Vec<Value> {
    let text = include_str!("../../../../../contracts/fixtures/phase1.json");
    let fixtures: Value = serde_json::from_str(text).expect("fixtures");
    let sites = fixtures["sites"].as_array().expect("sites");
    let devices = fixtures["devices"].as_array().expect("devices");
    devices
        .iter()
        .map(|device| {
            let site = sites
                .iter()
                .find(|site| site["id"] == device["siteId"])
                .expect("the device's site");
            json!({
                "id": device["id"], "name": device["name"], "host": device["host"],
                "ip": device["ip"], "kind": device["kind"], "os": device["os"],
                "status": device["expectedStatus"], "tags": device["tags"],
                "maintenance": device["maintenance"],
                "site": { "id": site["id"], "name": site["name"] },
            })
        })
        .collect()
}

/// A `devices` answer holding `nodes`.
pub fn page(nodes: &[Value], next: Option<&str>, total: usize) -> Value {
    let edges: Vec<Value> = nodes
        .iter()
        .map(|node| json!({ "cursor": format!("c-{}", node["id"].as_str().unwrap_or("")), "node": node }))
        .collect();
    data(json!({ "devices": {
        "edges": edges,
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
        "totalCount": total,
    }}))
}

/// A `device` answer for `node` with every field `get.graphql` asks for.
pub fn detail(node: &Value) -> Value {
    let mut device = node.clone();
    let extra = json!({
        "distribution": "debian", "macAddress": null, "metricsSource": "SSH",
        "metricsStale": false, "daemonVersion": null, "sessionLimit": null, "accessWarnings": [],
        "ssh": { "enabled": true, "port": 22, "username": "operator", "credentialStored": true,
                 "hostKey": { "algorithm": "ssh-ed25519", "fingerprintSha256": "SHA256:abc" } },
        "rdp": { "enabled": false, "port": 3389, "username": "", "credentialStored": false },
        "vnc": { "enabled": false, "port": 5900, "credentialStored": false },
        "checks": [{ "id": "0190a000-0000-7000-8003-000000000001", "name": "ping", "type": "PING",
                     "status": "UP", "paused": false }],
        "uptime": 0.9991, "batteries": [],
        "createdAt": "2026-11-01T09:00:00+00:00", "updatedAt": "2026-11-02T09:00:00+00:00",
    });
    for (key, value) in extra.as_object().expect("an object") {
        device[key] = value.clone();
    }
    data(json!({ "device": device }))
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

#[tokio::test]
async fn cmd_020_device_list_prints_a_table_for_a_person() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceList", page(&devices, None, devices.len()));
    let run = signed_in()
        .run(&["device", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let first = run.stdout.lines().next().expect("a header");
    assert!(first.starts_with("NAME") && first.contains("STATUS") && first.contains("SITE"));
    for device in &devices {
        let line = run
            .stdout
            .lines()
            .find(|line| line.starts_with(device["name"].as_str().expect("name")))
            .expect("a row for each device");
        assert!(line.contains(device["id"].as_str().expect("id")), "{line}");
    }
    assert_eq!(run.stderr, "", "no message when there is no next page");
    let seen = hub.seen();
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some("Bearer login-token-for-device-tests")
    );
    assert_eq!(seen[0].variables["first"], 50);
    assert_eq!(seen[0].variables["sort"], "NAME");
}

#[tokio::test]
async fn cmd_020_device_list_json_keeps_the_k1_field_names() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer(
        "DeviceList",
        page(&devices[..2], Some("next-1"), devices.len()),
    );
    let run = signed_in()
        .run(&["device", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let json = run.json();
    assert_eq!(json["totalCount"], devices.len());
    assert_eq!(
        json["pageInfo"],
        json!({ "hasNextPage": true, "endCursor": "next-1" })
    );
    assert_eq!(json["edges"][0]["node"]["name"], devices[0]["name"]);
    assert_eq!(
        json["edges"][1]["node"]["site"]["name"],
        devices[1]["site"]["name"]
    );
    assert_eq!(run.stderr, "", "with --json the next-page note stays off");
}

#[tokio::test]
async fn cmd_026_all_follows_every_page_and_limit_and_after_pass_through() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer(
        "DeviceList",
        page(&devices[..2], Some("cursor-1"), devices.len()),
    )
    .answer("DeviceList", page(&devices[2..], None, devices.len()));
    let run = signed_in()
        .run(&["device", "list", "--all", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let edges = run.json()["edges"].as_array().expect("edges").len();
    assert_eq!(edges, devices.len());
    let seen = hub.seen();
    assert_eq!(
        (
            seen[0].variables["first"].clone(),
            seen[1].variables["after"].clone()
        ),
        (json!(200), json!("cursor-1"))
    );

    let page_hub = FakeHub::start().await;
    page_hub.answer(
        "DeviceList",
        page(&devices[..1], Some("cursor-2"), devices.len()),
    );
    let run = signed_in()
        .run(&[
            "device",
            "list",
            "--limit",
            "1",
            "--after",
            "cursor-1",
            "--hub",
            &page_hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stderr.contains("--after cursor-2"), "{}", run.stderr);
    let seen = page_hub.seen();
    assert_eq!(
        (
            seen[0].variables["first"].clone(),
            seen[0].variables["after"].clone()
        ),
        (json!(1), json!("cursor-1"))
    );
}

#[tokio::test]
async fn cmd_026_filters_and_order_reach_the_hub_and_a_site_name_becomes_its_id() {
    let hub = FakeHub::start().await;
    let site = json!({ "id": "0190a000-0000-7000-8001-000000000001", "name": "home" });
    hub.answer("SiteNames", data(json!({ "sites": [site] })))
        .answer("DeviceList", page(&[], None, 0));
    let args = [
        "device",
        "list",
        "--status",
        "down",
        "--status",
        "degraded",
        "--tag",
        "infra",
        "--name",
        "pve",
        "--site",
        "home",
        "--sort",
        "status",
        "--hub",
        &hub.origin,
    ];
    let run = signed_in().run(&args).await;
    assert_eq!(run.code, 0, "{run:?}");
    let list = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "DeviceList")
        .expect("a list");
    assert_eq!(
        list.variables["filter"],
        json!({ "status": ["DOWN", "DEGRADED"], "tags": ["infra"], "name": "pve", "siteId": site["id"] })
    );
    assert_eq!(list.variables["sort"], "STATUS");
}
