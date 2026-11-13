//! Saved permissions per hub, and flags that override them for one run (MCP-002).

use serde_json::Value;

use super::session::{Desk, LOGIN_TOKEN, hub};

#[tokio::test]
async fn mcp_002_saved_write_for_checks_lists_check_changes_and_no_device_changes() {
    let hub = hub().await;
    let desk = Desk::new();
    let (code, stdout, stderr) = desk
        .run(
            LOGIN_TOKEN,
            &[
                "mcp",
                "permissions",
                "set",
                "--write",
                "checks",
                "--hub",
                &hub.origin,
            ],
        )
        .await;
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("checks"), "{stdout}");
    let (mut session, initialised) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let instructions = initialised["instructions"].as_str().unwrap_or_default();
    assert!(
        instructions.contains("May change things: checks."),
        "{instructions}"
    );
    let names = session.tool_names().await;
    assert!(
        names.contains(&"anima_check_update".to_owned()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"anima_device_create".to_owned()),
        "{names:?}"
    );
    session.finish().await;

    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &["--read-only"]).await;
    let names = session.tool_names().await;
    assert!(
        !names.contains(&"anima_check_update".to_owned()),
        "a flag overrides"
    );
    session.finish().await;

    let (mut session, _) = desk
        .start(&hub, LOGIN_TOKEN, &["--allow-write", "devices"])
        .await;
    let names = session.tool_names().await;
    assert!(
        names.contains(&"anima_device_create".to_owned()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"anima_check_update".to_owned()),
        "{names:?}"
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_002_read_only_on_permissions_set_clears_write_and_show_says_so() {
    let desk = Desk::new();
    let hub_flag = ["--hub", "https://hub.example"];
    let set = |extra: &'static [&'static str]| {
        let mut args = vec!["mcp", "permissions", "set"];
        args.extend_from_slice(extra);
        args.extend_from_slice(&hub_flag);
        args
    };
    let (code, _, stderr) = desk
        .run(
            LOGIN_TOKEN,
            &set(&["--toolsets", "devices,checks", "--write"]),
        )
        .await;
    assert_eq!(code, 0, "{stderr}");
    let (code, _, stderr) = desk.run(LOGIN_TOKEN, &set(&["--read-only"])).await;
    assert_eq!(code, 0, "{stderr}");
    let (code, stdout, stderr) = desk
        .run(
            LOGIN_TOKEN,
            &[
                "mcp",
                "permissions",
                "show",
                "--json",
                "--hub",
                "https://hub.example",
            ],
        )
        .await;
    assert_eq!(code, 0, "{stderr}");
    let shown: Value = serde_json::from_str(&stdout).expect("JSON");
    assert_eq!(shown["toolsets"], serde_json::json!(["devices", "checks"]));
    assert_eq!(shown["write"], serde_json::json!([]));
    let (_, words, _) = desk
        .run(
            LOGIN_TOKEN,
            &["mcp", "permissions", "show", "--hub", "https://hub.example"],
        )
        .await;
    assert!(words.contains("May change things: none."), "{words}");
    assert!(words.contains("read scope"), "{words}");
}
