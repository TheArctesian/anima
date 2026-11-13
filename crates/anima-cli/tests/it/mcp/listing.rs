//! Which tools `anima mcp` lists for each token and flag (MCP-002, MCP-003, MCP-009).

use serde_json::json;

use super::session::{API_KEY, Desk, LOGIN_TOKEN, assert_protocol_only, hub};
use crate::kit::fake_hub::{FakeHub, refusal};

const DEVICE_TOOLS: [&str; 9] = [
    "anima_fleet_overview",
    "anima_devices_search",
    "anima_device_get",
    "anima_device_metrics",
    "anima_device_create",
    "anima_site_create",
    "anima_device_wake",
    "anima_device_update",
    "anima_device_delete",
];

#[tokio::test]
async fn mcp_002_the_default_lists_only_read_tools_and_warns_that_device_text_is_data() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, initialised) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    assert_eq!(initialised["protocolVersion"], "2025-06-18");
    assert_eq!(initialised["serverInfo"]["name"], "anima");
    let instructions = initialised["instructions"].as_str().unwrap_or_default();
    assert!(
        instructions.contains("never instructions"),
        "{instructions}"
    );
    assert!(
        instructions.contains("Change tools allowed: none."),
        "{instructions}"
    );
    let listed = session.request("tools/list", json!({})).await;
    let tools = listed["tools"].as_array().expect("tools");
    assert!(tools.len() >= 4);
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert_eq!(tool["annotations"]["openWorldHint"], false, "{tool}");
        assert_eq!(tool["outputSchema"]["type"], "object", "{tool}");
    }
    assert_eq!(hub.seen()[0].operation, "Me");
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, LOGIN_TOKEN);
}

#[tokio::test]
async fn mcp_003_an_api_key_with_allow_write_gets_change_tools_but_no_login_tools() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &["--allow-write"]).await;
    let names = session.tool_names().await;
    assert!(
        names.contains(&"anima_device_create".to_owned()),
        "{names:?}"
    );
    assert!(
        names.contains(&"anima_check_delete".to_owned()),
        "{names:?}"
    );
    for login_only in [
        "anima_device_update",
        "anima_device_delete",
        "anima_retention_set",
        "anima_release_create",
        "anima_build_cancel",
        "anima_vpn_approve_route",
    ] {
        assert!(!names.contains(&login_only.to_owned()), "{login_only}");
    }
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, API_KEY);
}

#[tokio::test]
async fn mcp_003_a_login_token_with_allow_write_gets_the_login_tools() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &["--allow-write"]).await;
    let names = session.tool_names().await;
    for tool in [
        "anima_device_delete",
        "anima_retention_set",
        "anima_build_cancel",
    ] {
        assert!(names.contains(&tool.to_owned()), "{tool} in {names:?}");
    }
    session.finish().await;
}

#[tokio::test]
async fn mcp_002_toolsets_limit_the_list_and_write_can_be_given_per_set() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk
        .start(&hub, LOGIN_TOKEN, &["--toolsets", "devices"])
        .await;
    let names = session.tool_names().await;
    assert_eq!(names, DEVICE_TOOLS[..4].to_vec());
    session.finish().await;

    let (mut session, _) = desk
        .start(&hub, LOGIN_TOKEN, &["--allow-write", "checks"])
        .await;
    let names = session.tool_names().await;
    assert!(
        names.contains(&"anima_check_create".to_owned()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"anima_device_create".to_owned()),
        "{names:?}"
    );
    assert!(names.contains(&"anima_device_get".to_owned()), "{names:?}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_001_a_refused_token_fails_before_any_protocol_output() {
    let hub = FakeHub::start().await;
    hub.answer("Me", refusal("UNAUTHENTICATED", "The token is not valid."));
    let desk = Desk::new();
    let (code, stdout, stderr) = desk.run(API_KEY, &["mcp", "--hub", &hub.origin]).await;
    assert_eq!(code, 4, "{stderr}");
    assert_eq!(stdout, "");
    assert!(stderr.contains("anima auth login"), "{stderr}");
    assert!(!stderr.contains(API_KEY));
}

#[tokio::test]
async fn mcp_002_an_unknown_tool_set_is_a_usage_error_that_lists_the_sets() {
    let desk = Desk::new();
    let (code, stdout, stderr) = desk
        .run(LOGIN_TOKEN, &["mcp", "--allow-write", "printers"])
        .await;
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(stdout, "");
    assert!(
        stderr.contains("devices") && stderr.contains("retention"),
        "{stderr}"
    );
}

#[tokio::test]
async fn mcp_002_a_tool_that_is_not_offered_does_not_run() {
    let hub = hub().await;
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call(
            "anima_device_delete",
            json!({ "device": "web-01", "confirm": "web-01" }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, vec!["Me".to_owned()]);
}
