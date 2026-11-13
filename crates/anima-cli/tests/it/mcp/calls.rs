//! Tool calls: structured results, hub errors as tool errors, and deletes that need the exact
//! name (MCP-007, MCP-009, MCP-010).

use serde_json::json;

use super::session::{Desk, LOGIN_TOKEN, assert_protocol_only, hub};
use crate::fleet::devices::{fixture_devices, page};
use crate::kit::fake_hub::{data, refusal};

const MISSING: &str = "0190a000-0000-7000-8001-0000000000ff";

#[tokio::test]
async fn mcp_009_devices_search_returns_names_and_ids_as_structured_content() {
    let hub = hub().await;
    let devices = fixture_devices();
    hub.answer(
        "DeviceList",
        page(&devices[..2], Some("c-next"), devices.len()),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call("anima_devices_search", json!({ "name": "a", "limit": 2 }))
        .await;
    assert_eq!(answer["isError"], false, "{answer}");
    let found = &answer["structuredContent"];
    assert_eq!(found["devices"][0]["name"], devices[0]["name"]);
    assert_eq!(found["devices"][0]["id"], devices[0]["id"]);
    assert!(
        found["devices"][0].get("host").is_none(),
        "short by default"
    );
    assert_eq!(found["nextCursor"], "c-next");
    assert_eq!(found["totalCount"], devices.len());
    let sent = hub.seen().pop().expect("a search");
    assert_eq!(sent.variables["first"], 2);
    assert_eq!(
        sent.authorization.as_deref(),
        Some("Bearer login-token-for-mcp-tests")
    );
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, LOGIN_TOKEN);
}

#[tokio::test]
async fn mcp_010_a_hub_not_found_is_a_tool_error_that_says_what_to_call_next() {
    let hub = hub().await;
    hub.answer("DeviceGet", refusal("NOT_FOUND", "No such device."));
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call("anima_device_get", json!({ "device": MISSING }))
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    let text = answer["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("anima_devices_search"), "{text}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_010_wrong_arguments_are_a_tool_error_and_reach_no_hub() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call(
            "anima_device_get",
            json!({ "device": "a", "password": "x" }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    session.finish().await;
    assert_eq!(hub.seen().len(), 1, "only the token check");
}

#[tokio::test]
async fn mcp_007_a_delete_without_the_exact_name_never_reaches_the_hub() {
    let hub = hub().await;
    let devices = fixture_devices();
    let name = devices[0]["name"].as_str().expect("a name").to_owned();
    let id = devices[0]["id"].as_str().expect("an id").to_owned();
    hub.answer(
        "DeviceUptime",
        data(json!({ "device": { "id": id, "name": name, "uptime": null } })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &["--allow-write"]).await;
    let by_name = session
        .call(
            "anima_device_delete",
            json!({ "device": name, "confirm": "yes" }),
        )
        .await;
    assert_eq!(by_name["isError"], true, "{by_name}");
    let by_id = session
        .call(
            "anima_device_delete",
            json!({ "device": id, "confirm": "yes" }),
        )
        .await;
    assert_eq!(by_id["isError"], true, "{by_id}");
    let text = by_id["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("Nothing was deleted"), "{text}");
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert!(
        !operations.contains(&"DeviceDelete".to_owned()),
        "{operations:?}"
    );
    assert!(
        !operations.contains(&"DeviceList".to_owned()),
        "a name is checked first"
    );
}
