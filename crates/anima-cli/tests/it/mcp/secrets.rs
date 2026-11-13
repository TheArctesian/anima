//! No tool takes or returns a secret (MCP-004), and a read-scope key's refusal says what the
//! key lacks (MCP-003, MCP-010).

use serde_json::{Value, json};

use super::session::{API_KEY, Desk, LOGIN_TOKEN, assert_protocol_only, hub};
use crate::kit::fake_hub::{data, refusal};

const DEVICE: &str = "0190a000-0000-7000-8001-000000000001";
const CHECK: &str = "0190a000-0000-7000-8003-000000000001";

fn text(answer: &Value) -> String {
    answer["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn mcp_004_a_check_with_a_bearer_token_or_community_is_refused_before_the_hub() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &["--allow-write"]).await;
    for config in [
        json!({ "http": { "url": "https://nas.lan", "auth": { "bearerToken": "s3cret-bearer" } } }),
        json!({ "snmp": { "oid": "1.3.6.1", "community": "s3cret-bearer" } }),
    ] {
        let answer = session
            .call(
                "anima_check_create",
                json!({ "device": DEVICE, "name": "web", "config": config }),
            )
            .await;
        assert_eq!(answer["isError"], true, "{answer}");
        assert!(text(&answer).contains("--from-file"), "{answer}");
        assert!(!text(&answer).contains("s3cret-bearer"), "{answer}");
    }
    let (printed, stderr) = session.finish().await;
    assert_eq!(hub.seen().len(), 1, "only the token check reached the hub");
    assert_protocol_only(&printed, &stderr, API_KEY);
}

#[tokio::test]
async fn mcp_003_a_read_scope_key_refused_by_the_hub_hears_what_the_key_lacks() {
    let hub = hub().await;
    hub.answer("CheckSetPaused", refusal("FORBIDDEN", "Not permitted."));
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &["--allow-write"]).await;
    let answer = session
        .call(
            "anima_check_set_paused",
            json!({ "check": CHECK, "paused": true }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    assert!(text(&answer).contains("read scope"), "{answer}");
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, API_KEY);
}

#[tokio::test]
async fn mcp_004_check_get_never_returns_the_push_path() {
    let hub = hub().await;
    let check = json!({
        "id": CHECK, "name": "backup", "type": "PUSH", "status": "UP", "paused": false,
        "invert": false, "intervalSeconds": 60, "retries": 3, "retryIntervalSeconds": 20,
        "degradedThresholdMs": 1000, "certificateDaysLeft": null, "domainDaysLeft": null,
        "device": { "id": DEVICE, "name": "nas" }, "parent": null,
        "config": { "__typename": "PushConfig", "pushPath": "/push/s3cret-push-token" },
        "notifications": { "resendEveryBeats": 0, "channels": [] },
        "lastHeartbeat": null, "uptime": 1.0,
    });
    hub.answer("CheckGet", data(json!({ "check": check })));
    hub.answer(
        "CheckHeartbeats",
        data(json!({ "check": { "heartbeats": { "edges": [],
            "pageInfo": { "hasNextPage": false, "endCursor": null } } } })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call("anima_check_get", json!({ "check": CHECK, "detail": true }))
        .await;
    assert_eq!(answer["isError"], false, "{answer}");
    assert_eq!(answer["structuredContent"]["check"]["name"], "backup");
    let (printed, _) = session.finish().await;
    assert!(
        printed
            .iter()
            .all(|line| !line.contains("s3cret-push-token")),
        "{printed:?}"
    );
}

#[tokio::test]
async fn mcp_009_an_unknown_argument_beside_a_page_is_refused() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call(
            "anima_events_search",
            json!({ "limit": 5, "password": "x" }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    session.finish().await;
    assert_eq!(hub.seen().len(), 1, "only the token check");
}
