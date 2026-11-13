//! Per-tool states saved per hub, the bulk forms, and the flags at start that combine with
//! them (MCP-002, MCP-003, MCP-012, MCP-014, MCP-015).

use serde_json::{Value, json};

use super::session::{API_KEY, Desk, LOGIN_TOKEN, hub};

const ORIGIN: &str = "https://hub.example";

/// `anima mcp permissions <args> --hub https://hub.example`.
async fn permissions(desk: &Desk, args: &[&str]) -> (i32, String, String) {
    let mut all = vec!["mcp", "permissions"];
    all.extend_from_slice(args);
    all.extend_from_slice(&["--hub", ORIGIN]);
    desk.run(LOGIN_TOKEN, &all).await
}

/// Each tool's state in `permissions show --json`.
async fn states(desk: &Desk) -> Value {
    let (code, stdout, stderr) = permissions(desk, &["show", "--json"]).await;
    assert_eq!(code, 0, "{stderr}");
    serde_json::from_str(&stdout).expect("JSON")
}

fn state_of<'a>(shown: &'a Value, tool: &str) -> &'a Value {
    let tools = shown["tools"].as_array().expect("tools");
    &tools
        .iter()
        .find(|row| row["tool"] == tool)
        .expect("the tool")["state"]
}

#[tokio::test]
async fn mcp_014_show_json_has_one_stable_row_per_tool_and_reads_as_read_only_by_default() {
    let desk = Desk::new();
    let shown = states(&desk).await;
    assert_eq!(shown["hub"], ORIGIN);
    let row = shown["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|row| row["tool"] == "anima_retention_set")
        .expect("a row")
        .clone();
    let mut keys: Vec<&str> = row
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "description",
            "kind",
            "needsSignIn",
            "state",
            "tool",
            "toolset"
        ]
    );
    assert_eq!(row["toolset"], "retention");
    assert_eq!(row["kind"], "change");
    assert_eq!(row["needsSignIn"], true);
    assert_eq!(row["state"], "off");
    assert_eq!(state_of(&shown, "anima_device_get"), "allow");
    let summary = shown["summary"].as_str().unwrap_or_default();
    assert!(summary.contains("Change tools allowed: none."), "{summary}");
    let (_, table, _) = permissions(&desk, &["show"]).await;
    assert!(
        table.contains("TOOL SET") && table.contains("NEEDS SIGN-IN"),
        "{table}"
    );
}

#[tokio::test]
async fn mcp_014_saved_states_show_then_narrow_anywhere_then_reset() {
    let desk = Desk::new();
    desk.save_permissions(
        ORIGIN,
        &json!({ "change": "ask", "tools": { "anima_check_update": "allow" } }),
    );
    let shown = states(&desk).await;
    assert_eq!(state_of(&shown, "anima_check_update"), "allow");
    assert_eq!(state_of(&shown, "anima_check_delete"), "ask");
    assert_eq!(state_of(&shown, "anima_session_end"), "ask");
    assert_eq!(state_of(&shown, "anima_device_get"), "allow");
    let (code, _, stderr) = permissions(&desk, &["set", "--off", "anima_check_update"]).await;
    assert_eq!(code, 0, "{stderr}");
    let (code, _, stderr) = permissions(&desk, &["set", "--read-only"]).await;
    assert_eq!(code, 0, "{stderr}");
    let shown = states(&desk).await;
    assert_eq!(state_of(&shown, "anima_check_update"), "off");
    assert_eq!(state_of(&shown, "anima_session_end"), "off");
    let (code, _, stderr) = permissions(&desk, &["set", "--off", "vpn,events"]).await;
    assert_eq!(code, 0, "{stderr}");
    let shown = states(&desk).await;
    assert_eq!(state_of(&shown, "anima_vpn_status"), "off");
    assert_eq!(state_of(&shown, "anima_audit_list"), "off");
    let (code, _, stderr) = permissions(&desk, &["reset"]).await;
    assert_eq!(code, 2, "a reset that turns vpn back on widens: {stderr}");
    assert_eq!(state_of(&states(&desk).await, "anima_vpn_status"), "off");

    let desk = Desk::new();
    desk.save_permissions(ORIGIN, &json!({ "change": "ask" }));
    let (code, _, stderr) = permissions(&desk, &["reset"]).await;
    assert_eq!(code, 0, "a reset that only narrows: {stderr}");
    assert_eq!(state_of(&states(&desk).await, "anima_check_delete"), "off");
}

#[tokio::test]
async fn mcp_014_f4_a_widening_without_a_terminal_exits_2_even_with_yes() {
    let desk = Desk::new();
    for args in [
        &["set", "--allow-all"][..],
        &["set", "--allow-all", "--yes"],
        &["set", "--allow-all", "--toolset", "checks", "--yes"],
        &["set", "--ask-all", "--kind", "change"],
        &["set", "--allow", "anima_check_delete", "--yes"],
        &["set", "--ask", "checks"],
    ] {
        let (code, stdout, stderr) = permissions(&desk, args).await;
        assert_eq!(code, 2, "{args:?}: {stderr}");
        assert_eq!(stdout, "", "{args:?}");
        assert!(
            stderr.contains("terminal") && stderr.contains("Settings"),
            "{args:?}: {stderr}"
        );
    }
    let shown = states(&desk).await;
    assert_eq!(state_of(&shown, "anima_check_delete"), "off");
    let summary = shown["summary"].as_str().unwrap_or_default();
    assert!(summary.contains("Change tools allowed: none."), "{summary}");
    let (code, _, stderr) = permissions(&desk, &["set", "--allow-all", "--kind", "read"]).await;
    assert_eq!(code, 0, "read tools are allowed already: {stderr}");
}

#[tokio::test]
async fn mcp_014_an_unknown_name_exits_2_and_lists_the_valid_names() {
    let desk = Desk::new();
    let (code, stdout, stderr) = permissions(&desk, &["set", "--allow", "printers"]).await;
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(stdout, "");
    assert!(
        stderr.contains("retention") && stderr.contains("anima_session_end"),
        "{stderr}"
    );
    let (code, _, stderr) = permissions(&desk, &["set"]).await;
    assert_eq!(code, 2, "{stderr}");
}

#[tokio::test]
async fn mcp_012_a_tool_saved_off_is_not_listed_and_a_call_never_reaches_the_hub() {
    let hub = hub().await;
    let desk = Desk::new();
    let (code, _, stderr) = desk
        .run(
            LOGIN_TOKEN,
            &[
                "mcp",
                "permissions",
                "set",
                "--off",
                "anima_vpn_status",
                "--hub",
                &hub.origin,
            ],
        )
        .await;
    assert_eq!(code, 0, "{stderr}");
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &["--allow-write"]).await;
    let names = session.tool_names().await;
    assert!(!names.contains(&"anima_vpn_status".to_owned()), "{names:?}");
    let answer = session.call("anima_vpn_status", json!({})).await;
    assert_eq!(answer["isError"], true, "{answer}");
    let text = answer["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("ask the operator"), "{text}");
    assert!(!text.contains("anima mcp"), "{text}");
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, vec!["Me".to_owned()]);
}

#[tokio::test]
async fn mcp_015_flags_narrow_saved_states_and_allow_write_raises_only_unsaved_tools() {
    let hub = hub().await;
    let desk = Desk::new();
    desk.save_permissions(
        &hub.origin,
        &json!({ "tools": { "anima_check_delete": "ask", "anima_check_set_paused": "off",
                            "anima_device_wake": "allow" } }),
    );
    let (mut session, initialised) = desk.start(&hub, LOGIN_TOKEN, &["--allow-write"]).await;
    let names = session.tool_names().await;
    assert!(names.contains(&"anima_check_update".to_owned()), "raised");
    assert!(
        names.contains(&"anima_check_delete".to_owned()),
        "ask stays"
    );
    assert!(
        !names.contains(&"anima_check_set_paused".to_owned()),
        "off wins"
    );
    let instructions = initialised["instructions"].as_str().unwrap_or_default();
    assert!(
        instructions.contains("before each call: anima_check_delete."),
        "{instructions}"
    );
    session.finish().await;

    let (mut session, _) = desk
        .start(&hub, LOGIN_TOKEN, &["--read-only", "--toolsets", "checks"])
        .await;
    let names = session.tool_names().await;
    assert_eq!(names, vec!["anima_check_get".to_owned()]);
    session.finish().await;
}

#[tokio::test]
async fn mcp_003_an_api_key_never_lists_a_sign_in_tool_even_when_saved_allow() {
    let hub = hub().await;
    let desk = Desk::new();
    desk.save_permissions(&hub.origin, &json!({ "read": "allow", "change": "allow" }));
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let names = session.tool_names().await;
    assert!(names.contains(&"anima_session_end".to_owned()), "{names:?}");
    for sign_in in [
        "anima_device_update",
        "anima_device_delete",
        "anima_retention_set",
        "anima_release_create",
        "anima_build_cancel",
        "anima_vpn_approve_route",
    ] {
        assert!(!names.contains(&sign_in.to_owned()), "{sign_in}");
    }
    let answer = session
        .call(
            "anima_retention_set",
            json!({ "subject": "EVENTS", "days": 1 }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    let said = answer["content"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("needs the operator's own sign-in"), "{said}");
    session.finish().await;
    assert_eq!(hub.seen().len(), 1, "only the token check");
}
