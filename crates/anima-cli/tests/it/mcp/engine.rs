//! The `extensions`, `sources` and `projects` tool sets against the scripted hub (MCP-002,
//! MCP-004, MCP-007, MCP-008, MCP-017): the extension tools and the rules that hold for all
//! three sets. The tools of sources and projects are in `engine_sources` and
//! `engine_projects`.

use serde_json::{Value, json};

use super::session::{Desk, LOGIN_TOKEN, Session, hub};
use crate::kit::fake_hub::{FakeHub, data};

pub const TOOLS: [&str; 10] = [
    "anima_extensions_list",
    "anima_extension_set",
    "anima_sources_overview",
    "anima_source_account_sync",
    "anima_source_account_set_paused",
    "anima_source_account_remove",
    "anima_projects_search",
    "anima_project_get",
    "anima_repository_set_left_out",
    "anima_project_resource_confirm",
];

pub async fn writing(hub: &FakeHub, desk: &Desk) -> Session {
    desk.start(hub, LOGIN_TOKEN, &["--allow-write"]).await.0
}

pub fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["isError"], false, "{answer}");
    &answer["structuredContent"]
}

pub fn sent(hub: &FakeHub, operation: &str) -> Vec<Value> {
    hub.seen()
        .into_iter()
        .filter(|seen| seen.operation == operation)
        .map(|seen| seen.variables)
        .collect()
}

fn extension(key: &str, on: bool, needs: &[&str]) -> Value {
    json!({ "key": key, "name": key, "dataClass": "A", "built": true, "fixed": false,
            "on": on, "needs": needs, "worksWith": [], "neededBy": [], "navigation": null,
            "changedAt": null })
}

#[tokio::test]
async fn mcp_017_extensions_list_returns_every_extension() {
    let hub = hub().await;
    hub.answer(
        "ExtensionList",
        data(json!({ "extensions": [extension("projects", false, &["github"])] })),
    );
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session.call("anima_extensions_list", json!({})).await;
    assert_eq!(structured(&answer)["extensions"][0]["key"], "projects");
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_extension_set_says_which_other_extensions_it_turned_on() {
    let hub = hub().await;
    hub.answer(
        "ExtensionList",
        data(
            json!({ "extensions": [extension("projects", false, &["github"]),
                                    extension("github", false, &[])] }),
        ),
    );
    hub.answer(
        "ExtensionSet",
        data(
            json!({ "setExtension": [extension("projects", true, &["github"]),
                                      extension("github", true, &[])] }),
        ),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_extension_set",
            json!({ "extension": "projects", "on": true }),
        )
        .await;
    let result = structured(&answer);
    assert_eq!(result["alsoChanged"], json!(["github"]));
    let note = result["note"].as_str().unwrap_or_default();
    assert!(note.contains("also turned github on"), "{note}");
    let unknown = session
        .call(
            "anima_extension_set",
            json!({ "extension": "nope", "on": true }),
        )
        .await;
    assert_eq!(unknown["isError"], true, "{unknown}");
    assert_eq!(sent(&hub, "ExtensionSet").len(), 1);
    session.finish().await;
}

#[tokio::test]
async fn mcp_004_no_tool_of_the_three_sets_takes_a_key_and_the_github_app_has_no_tool() {
    let hub = hub().await;
    let mut session = writing(&hub, &Desk::new()).await;
    let listed = session.request("tools/list", json!({})).await;
    let tools = listed["tools"].as_array().expect("tools");
    for name in TOOLS {
        assert!(tools.iter().any(|tool| tool["name"] == name), "{name}");
    }
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or_default();
        assert!(
            !name.contains("connect") && !name.contains("watch"),
            "{name}"
        );
        let inputs = tool["inputSchema"]["properties"].as_object();
        for field in inputs.into_iter().flat_map(|fields| fields.keys()) {
            let lower = field.to_lowercase();
            assert!(
                lower != "key"
                    && ![
                        "privatekey",
                        "secret",
                        "password",
                        "token",
                        "credential",
                        "pem"
                    ]
                    .iter()
                    .any(|word| lower.contains(word)),
                "{name} takes {field}"
            );
        }
    }
    session.finish().await;
}

#[tokio::test]
async fn mcp_002_the_change_tools_of_the_three_sets_are_off_until_allowed() {
    let hub = hub().await;
    let desk = Desk::new();
    let mut session = desk.start(&hub, LOGIN_TOKEN, &[]).await.0;
    let names = session.tool_names().await;
    for (tool, change) in [
        ("anima_extensions_list", false),
        ("anima_extension_set", true),
        ("anima_sources_overview", false),
        ("anima_source_account_sync", true),
        ("anima_source_account_set_paused", true),
        ("anima_source_account_remove", true),
        ("anima_projects_search", false),
        ("anima_project_get", false),
        ("anima_repository_set_left_out", true),
        ("anima_project_resource_confirm", true),
    ] {
        assert_eq!(names.contains(&tool.to_owned()), !change, "{tool}");
    }
    let refused = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": "arctesian/anima", "leftOut": true }),
        )
        .await;
    assert_eq!(refused["isError"], true, "{refused}");
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, vec!["Me".to_owned()]);
}

#[tokio::test]
async fn mcp_002_the_catalogue_lists_the_three_sets_with_their_kinds() {
    let desk = Desk::new();
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
    let rows = shown["tools"].as_array().expect("tools");
    for name in TOOLS {
        let row = rows.iter().find(|row| row["tool"] == name).expect(name);
        let reads = name.contains("list")
            || name.contains("overview")
            || name.contains("search")
            || name.ends_with("_get");
        assert_eq!(row["kind"], if reads { "read" } else { "change" }, "{name}");
        assert_eq!(row["state"], if reads { "allow" } else { "off" }, "{name}");
    }
    for set in ["extensions", "sources", "projects"] {
        assert!(rows.iter().any(|row| row["toolset"] == set), "{set}");
    }
}
