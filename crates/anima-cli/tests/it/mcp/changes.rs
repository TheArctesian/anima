//! One call of each change tool T7 left without a test, with `--allow-write` and the login
//! token, against the scripted hub (MCP-007, MCP-009, MCP-017).

use serde_json::{Value, json};

use super::reads::{DEVICE, SESSION, session_row};
use super::session::{Desk, LOGIN_TOKEN, Session, assert_protocol_only, hub};
use crate::kit::fake_hub::{FakeHub, data};

const CHECK: &str = "0190a000-0000-7000-8003-000000000001";

async fn writing(hub: &FakeHub, desk: &Desk) -> Session {
    desk.start(hub, LOGIN_TOKEN, &["--allow-write"]).await.0
}

fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["isError"], false, "{answer}");
    &answer["structuredContent"]
}

fn sent(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .map_or_else(|| panic!("no {operation}"), |seen| seen.variables)
}

/// A `CheckGet` answer for the TCP check "SSH port".
fn check_get() -> Value {
    data(json!({ "check": {
        "id": CHECK, "name": "SSH port", "type": "TCP", "status": "UP", "paused": false,
        "invert": false, "intervalSeconds": 60, "retries": 3, "retryIntervalSeconds": 20,
        "degradedThresholdMs": 1000, "certificateDaysLeft": null, "domainDaysLeft": null,
        "device": { "id": DEVICE, "name": "web-1" }, "parent": null,
        "config": { "__typename": "TcpConfig", "port": 22, "timeoutMs": 5000 },
        "notifications": { "resendEveryBeats": 0, "channels": [] },
        "lastHeartbeat": null, "uptime": 1.0,
    }}))
}

#[tokio::test]
async fn mcp_009_check_update_sends_only_the_fields_given() {
    let hub = hub().await;
    hub.answer(
        "CheckUpdate",
        data(json!({ "updateCheck": {
            "id": CHECK, "name": "SSH", "type": "TCP", "status": "UP", "paused": false } })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let answer = session
        .call(
            "anima_check_update",
            json!({ "check": CHECK, "name": "SSH", "retries": 5 }),
        )
        .await;
    assert_eq!(structured(&answer)["check"]["name"], "SSH");
    let variables = sent(&hub, "CheckUpdate");
    assert_eq!(variables["id"], CHECK);
    assert_eq!(variables["input"]["name"], "SSH");
    assert_eq!(variables["input"]["retries"], 5);
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, LOGIN_TOKEN);
}

#[tokio::test]
async fn mcp_007_check_delete_runs_only_with_the_exact_name_as_confirm() {
    let hub = hub().await;
    hub.answer("CheckGet", check_get());
    hub.answer("CheckDelete", data(json!({ "deleteCheck": CHECK })));
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let wrong = session
        .call(
            "anima_check_delete",
            json!({ "check": CHECK, "confirm": "ssh port" }),
        )
        .await;
    assert_eq!(wrong["isError"], true, "{wrong}");
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "CheckDelete")
    );
    let right = session
        .call(
            "anima_check_delete",
            json!({ "check": CHECK, "confirm": "SSH port" }),
        )
        .await;
    assert_eq!(structured(&right)["deleted"], CHECK);
    assert_eq!(sent(&hub, "CheckDelete")["id"], CHECK);
    session.finish().await;
}

#[tokio::test]
async fn mcp_009_maintenance_create_sends_the_k1_window_input() {
    let hub = hub().await;
    hub.answer(
        "MaintenanceCreate",
        data(json!({ "createMaintenanceWindow": {
            "id": "w-1", "name": "Patch night", "kind": "MANUAL", "activeNow": false } })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let answer = session
        .call(
            "anima_maintenance_create",
            json!({ "name": "Patch night", "kind": "MANUAL", "timezone": "Europe/London",
                    "targets": [{ "type": "DEVICE", "target": DEVICE }] }),
        )
        .await;
    assert_eq!(structured(&answer)["window"]["id"], "w-1");
    let input = &sent(&hub, "MaintenanceCreate")["input"];
    assert_eq!(input["kind"], "MANUAL");
    assert_eq!(input["targets"][0]["target"], DEVICE);
    session.finish().await;
}

#[tokio::test]
async fn mcp_009_web_interface_create_adds_one_to_the_device() {
    let hub = hub().await;
    hub.answer(
        "WebCreate",
        data(json!({ "createWebInterface": {
            "id": "i-1", "deviceId": DEVICE, "label": "NAS", "url": "https://10.10.0.5:8443",
            "openIn": "ANIMA_TAB", "pinnedFingerprintSha256": null, "pinnedAt": null } })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let answer = session
        .call(
            "anima_web_interface_create",
            json!({ "device": DEVICE, "url": "https://10.10.0.5:8443", "label": "NAS" }),
        )
        .await;
    assert_eq!(structured(&answer)["webInterface"]["id"], "i-1");
    let variables = sent(&hub, "WebCreate");
    assert_eq!(variables["deviceId"], DEVICE);
    assert_eq!(variables["input"]["url"], "https://10.10.0.5:8443");
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_session_end_ends_a_live_session_and_refuses_a_non_id() {
    let hub = hub().await;
    hub.answer(
        "SessionEnd",
        data(json!({ "endSession": session_row(Some("OPERATOR")) })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let listed = session.request("tools/list", json!({})).await;
    let tool = listed["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "anima_session_end")
        .expect("listed with --allow-write")
        .clone();
    assert_eq!(tool["annotations"]["destructiveHint"], true);
    let wrong = session
        .call("anima_session_end", json!({ "session": "web-1" }))
        .await;
    assert_eq!(wrong["isError"], true, "{wrong}");
    let answer = session
        .call("anima_session_end", json!({ "session": SESSION }))
        .await;
    assert_eq!(structured(&answer)["session"]["endReason"], "OPERATOR");
    assert_eq!(sent(&hub, "SessionEnd")["id"], SESSION);
    session.finish().await;
}
