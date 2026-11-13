//! One call of each read tool T7 left without a test, against the scripted hub (MCP-009).

use serde_json::{Value, json};

use super::session::{API_KEY, Desk, assert_protocol_only, hub};
use crate::kit::fake_hub::data;

pub const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
pub const SESSION: &str = "0190a000-0000-7000-8009-000000000001";

/// A K1 `Session` as the session documents select it; ended when `reason` is given.
pub fn session_row(reason: Option<&str>) -> Value {
    json!({
        "id": SESSION, "deviceId": DEVICE, "deviceName": "web-1", "protocol": "SSH",
        "operatorName": "operator", "startedAt": "2026-11-13T09:00:00Z",
        "endedAt": reason.map(|_| "2026-11-13T09:05:00Z"), "endReason": reason,
        "clientAddr": "10.10.0.5",
    })
}

fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["isError"], false, "{answer}");
    &answer["structuredContent"]
}

#[tokio::test]
async fn mcp_017_sessions_list_pages_and_filters_live_sessions_of_a_device() {
    let hub = hub().await;
    hub.answer(
        "SessionList",
        data(json!({ "sessions": {
            "edges": [{ "cursor": "s1", "node": session_row(None) }],
            "pageInfo": { "hasNextPage": true, "endCursor": "s1" } } })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let answer = session
        .call(
            "anima_sessions_list",
            json!({ "device": DEVICE, "live": true, "limit": 1 }),
        )
        .await;
    let found = structured(&answer);
    assert_eq!(found["sessions"][0]["id"], SESSION);
    assert_eq!(found["sessions"][0]["endedAt"], Value::Null);
    assert_eq!(found["nextCursor"], "s1");
    let sent = hub.seen().pop().expect("a list");
    assert_eq!(sent.variables["deviceId"], DEVICE);
    assert_eq!(sent.variables["live"], true);
    assert_eq!(sent.variables["first"], 1);
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, API_KEY);
}

#[tokio::test]
async fn mcp_009_fleet_overview_counts_devices_and_names_those_that_need_attention() {
    let hub = hub().await;
    let count = |total: u32| json!({ "totalCount": total });
    hub.answer(
        "FleetOverview",
        data(json!({
            "all": count(3), "up": count(1), "down": count(1), "degraded": count(1),
            "maintenance": count(0), "unknown": count(0),
            "attention": { "totalCount": 2, "edges": [
                { "node": { "id": DEVICE, "name": "web-1", "status": "DOWN",
                            "site": { "id": "s-1", "name": "Home" } } }] },
            "sites": [{ "id": "s-1", "name": "Home" }],
        })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let answer = session.call("anima_fleet_overview", json!({})).await;
    let overview = structured(&answer).to_string();
    assert!(
        overview.contains("web-1") && overview.contains("Home"),
        "{overview}"
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_009_release_list_returns_releases_with_their_jobs() {
    let hub = hub().await;
    let release = json!({
        "id": "r-1", "version": "1.4.0", "commit": "0123abcd", "createdAt": "2026-11-12T10:00:00Z",
        "createdBy": "operator",
        "jobs": [{ "id": "j-1", "target": "LINUX_X86_64", "state": "PUBLISHED",
                   "failureReason": null }],
    });
    hub.answer(
        "ReleaseList",
        data(
            json!({ "releases": { "edges": [{ "cursor": "r1", "node": release }],
                                   "pageInfo": { "hasNextPage": false, "endCursor": "r1" } } }),
        ),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let answer = session.call("anima_releases_list", json!({})).await;
    let text = structured(&answer).to_string();
    assert!(
        text.contains("1.4.0") && text.contains("PUBLISHED"),
        "{text}"
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_009_vpn_status_gives_short_nodes_unless_detail() {
    let hub = hub().await;
    let node = json!({
        "name": "nas", "address": "100.64.0.2", "os": "linux", "role": "AGENT", "tags": [],
        "routes": [], "waitingRoutes": [], "online": true, "lastSeen": null, "keyExpiry": null,
        "keyState": "NEVER", "path": "DIRECT", "relayRegion": null, "inAnima": true,
    });
    hub.answer(
        "VpnStatus",
        data(json!({ "vpn": {
            "readAt": "2026-11-13T09:00:00Z", "fresh": true, "broker": "READY",
            "hub": node, "sites": [], "nodes": [node] } })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let answer = session.call("anima_vpn_status", json!({})).await;
    let vpn = &structured(&answer)["vpn"];
    assert_eq!(vpn["nodes"][0]["name"], "nas");
    assert!(vpn["nodes"][0].get("os").is_none(), "short by default");
    session.finish().await;
}

#[tokio::test]
async fn mcp_009_retention_list_returns_every_period() {
    let hub = hub().await;
    hub.answer(
        "RetentionList",
        data(
            json!({ "retention": [{ "subject": "EVENTS", "days": 90, "minDays": 7,
                                     "maxDays": 3650, "defaultDays": 90 }] }),
        ),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let answer = session.call("anima_retention_list", json!({})).await;
    let periods = &structured(&answer)["periods"];
    assert_eq!(periods[0]["subject"], "EVENTS");
    assert_eq!(periods[0]["days"], 90);
    session.finish().await;
}
