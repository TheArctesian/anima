//! The `extensions`, `sources` and `projects` tool sets against the scripted hub (MCP-002,
//! MCP-004, MCP-007, MCP-008, MCP-017): one test for each tool, and the rules that hold for all
//! of them.

use serde_json::{Value, json};

use super::session::{Desk, LOGIN_TOKEN, Session, hub};
use crate::kit::fake_hub::{FakeHub, data};

const ACCOUNT: &str = "0190a000-0000-7000-9001-000000000001";
const REPOSITORY: &str = "0190a000-0000-7000-9002-000000000001";

const TOOLS: [&str; 10] = [
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

async fn writing(hub: &FakeHub, desk: &Desk) -> Session {
    desk.start(hub, LOGIN_TOKEN, &["--allow-write"]).await.0
}

fn structured(answer: &Value) -> &Value {
    assert_eq!(answer["isError"], false, "{answer}");
    &answer["structuredContent"]
}

fn sent(hub: &FakeHub, operation: &str) -> Vec<Value> {
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

fn account(paused: bool) -> Value {
    json!({ "id": ACCOUNT, "sourceKey": "github", "label": "home-app", "instanceKey": "1/2",
            "credentialStored": true, "health": if paused { "PAUSED" } else { "OK" },
            "pausedBy": if paused { json!("OPERATOR") } else { Value::Null },
            "lastSyncAt": null, "lastError": null, "failureCount": 0, "nextSyncAt": null,
            "syncing": false, "createdAt": "2026-11-12T10:00:00Z" })
}

fn repository(description: &str, left_out: bool) -> Value {
    json!({ "id": REPOSITORY, "fullName": "arctesian/anima", "url": "https://github.com/a/b",
            "defaultBranch": "main", "private": true, "archived": false, "fork": false,
            "description": description, "pushedAt": null, "leftOut": left_out, "gone": false })
}

fn resource(state: &str) -> Value {
    json!({ "id": "res-1", "kind": "VERCEL", "state": state,
            "evidence": [{ "sign": "VERCEL_JSON", "path": "vercel.json" }],
            "foundAt": "2026-11-12T10:00:00Z", "confirmedAt": null })
}

fn project(description: &str) -> Value {
    json!({ "id": "p-1", "slug": "anima", "name": "Anima", "subjectKey": "k",
            "repositories": [repository(description, false)],
            "resources": [resource("FOUND_IN_REPOSITORY")], "syncedAt": null })
}

fn accounts_answer(hub: &FakeHub) {
    let run = json!({ "id": "run-1", "trigger": "MANUAL", "startedAt": "2026-11-12T10:00:00Z",
                      "endedAt": null, "outcome": "OK", "inserted": 1, "changed": 0,
                      "unchanged": 2, "removed": 0, "problems": [], "error": null });
    let mut row = account(false);
    row["runs"] = json!([run]);
    hub.answer("SourceAccounts", data(json!({ "sourceAccounts": [row] })));
}

fn projects_answer(hub: &FakeHub, description: &str) {
    hub.answer(
        "ProjectList",
        data(
            json!({ "projects": { "edges": [{ "cursor": "c1", "node": project(description) }],
                                   "pageInfo": { "hasNextPage": false, "endCursor": "c1" } } }),
        ),
    );
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
async fn mcp_017_sources_overview_returns_kinds_and_accounts_with_their_runs() {
    let hub = hub().await;
    hub.answer(
        "SourceKinds",
        data(json!({ "sources": [{ "key": "github", "name": "GitHub",
            "acquisition": "TOKEN_PULL", "scopes": ["metadata:read"], "writeReason": null,
            "gives": ["repositories"], "pollIntervalSeconds": 900, "consumers": ["projects"] }] })),
    );
    accounts_answer(&hub);
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session
        .call(
            "anima_sources_overview",
            json!({ "source": "github", "runs": 3 }),
        )
        .await;
    let result = structured(&answer);
    assert_eq!(result["sources"][0]["key"], "github");
    assert_eq!(result["accounts"][0]["label"], "home-app");
    assert_eq!(result["accounts"][0]["runs"][0]["inserted"], 1);
    let asked = &sent(&hub, "SourceAccounts")[0];
    assert_eq!(asked["sourceKey"], "github");
    assert_eq!(asked["runs"], 3);
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_source_account_sync_starts_a_sync_of_the_account_by_label() {
    let hub = hub().await;
    accounts_answer(&hub);
    let mut syncing = account(false);
    syncing["syncing"] = json!(true);
    hub.answer("SourceSync", data(json!({ "syncSourceAccount": syncing })));
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_source_account_sync",
            json!({ "account": "home-app" }),
        )
        .await;
    assert_eq!(structured(&answer)["account"]["syncing"], true);
    assert_eq!(sent(&hub, "SourceSync")[0]["id"], ACCOUNT);
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_source_account_set_paused_pauses_and_resumes() {
    let hub = hub().await;
    accounts_answer(&hub);
    hub.answer(
        "SourcePause",
        data(json!({ "setSourceAccountPaused": account(true) })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_source_account_set_paused",
            json!({ "account": ACCOUNT, "paused": true }),
        )
        .await;
    assert_eq!(structured(&answer)["account"]["health"], "PAUSED");
    let variables = &sent(&hub, "SourcePause")[0];
    assert_eq!(
        (&variables["id"], &variables["paused"]),
        (&json!(ACCOUNT), &json!(true))
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_007_source_account_remove_sends_nothing_without_the_label_and_a_records_choice() {
    let hub = hub().await;
    accounts_answer(&hub);
    hub.answer(
        "SourceRemove",
        data(json!({ "removeSourceAccount": ACCOUNT })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    for arguments in [
        json!({ "account": ACCOUNT, "confirm": "yes", "records": "KEEP" }),
        json!({ "account": ACCOUNT, "confirm": "", "records": "KEEP" }),
        json!({ "account": ACCOUNT, "confirm": "home-app" }),
        json!({ "account": ACCOUNT, "confirm": "HOME-APP", "records": "DELETE" }),
    ] {
        let refused = session.call("anima_source_account_remove", arguments).await;
        assert_eq!(refused["isError"], true, "{refused}");
    }
    assert!(sent(&hub, "SourceRemove").is_empty());
    let removed = session
        .call(
            "anima_source_account_remove",
            json!({ "account": ACCOUNT, "confirm": "home-app", "records": "DELETE" }),
        )
        .await;
    assert_eq!(structured(&removed)["removed"], ACCOUNT);
    let variables = sent(&hub, "SourceRemove");
    assert_eq!(variables.len(), 1);
    assert_eq!(variables[0]["deleteRecords"], true);
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_projects_search_returns_a_page_of_projects() {
    let hub = hub().await;
    projects_answer(&hub, "A device manager");
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session
        .call(
            "anima_projects_search",
            json!({ "leftOut": true, "limit": 5 }),
        )
        .await;
    let result = structured(&answer);
    assert_eq!(result["projects"][0]["slug"], "anima");
    assert_eq!(result["nextCursor"], Value::Null);
    let asked = &sent(&hub, "ProjectList")[0];
    assert_eq!(
        (&asked["leftOut"], &asked["first"]),
        (&json!(true), &json!(5))
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_project_get_returns_one_project_or_says_there_is_none() {
    let hub = hub().await;
    hub.answer("ProjectGet", data(json!({ "project": project("x") })));
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session
        .call("anima_project_get", json!({ "slug": "anima" }))
        .await;
    assert_eq!(
        structured(&answer)["project"]["resources"][0]["kind"],
        "VERCEL"
    );
    session.finish().await;

    let hub = self::hub().await;
    hub.answer("ProjectGet", data(json!({ "project": null })));
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let missing = session
        .call("anima_project_get", json!({ "slug": "nope" }))
        .await;
    assert_eq!(missing["isError"], true, "{missing}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_repository_set_left_out_leaves_out_and_restores_by_full_name() {
    let hub = hub().await;
    projects_answer(&hub, "x");
    let left = json!({ "id": "p-1", "slug": "anima", "name": "Anima", "subjectKey": "k",
                       "repositories": [repository("x", true)], "syncedAt": null });
    hub.answer(
        "RepositoryLeaveOut",
        data(json!({ "leaveOutRepository": left })),
    );
    hub.answer(
        "RepositoryRestore",
        data(json!({ "restoreRepository": project("x") })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let out = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": "arctesian/anima", "leftOut": true }),
        )
        .await;
    assert_eq!(
        structured(&out)["project"]["repositories"][0]["leftOut"],
        true
    );
    let back = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": REPOSITORY, "leftOut": false }),
        )
        .await;
    assert_eq!(structured(&back)["project"]["slug"], "anima");
    assert_eq!(sent(&hub, "RepositoryLeaveOut")[0]["id"], REPOSITORY);
    assert_eq!(sent(&hub, "RepositoryRestore")[0]["id"], REPOSITORY);
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_project_resource_confirm_confirms_the_resources_of_one_kind() {
    let hub = hub().await;
    hub.answer("ProjectGet", data(json!({ "project": project("x") })));
    let mut confirmed = resource("CONFIRMED");
    confirmed["confirmedAt"] = json!("2026-11-13T09:00:00Z");
    hub.answer(
        "ResourceConfirm",
        data(json!({ "confirmProjectResource": confirmed })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_project_resource_confirm",
            json!({ "slug": "anima", "kind": "VERCEL" }),
        )
        .await;
    assert_eq!(structured(&answer)["resources"][0]["state"], "CONFIRMED");
    assert_eq!(sent(&hub, "ResourceConfirm")[0]["id"], "res-1");
    let none = session
        .call(
            "anima_project_resource_confirm",
            json!({ "slug": "anima", "kind": "SERVER" }),
        )
        .await;
    assert_eq!(none["isError"], true, "{none}");
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
            json!({ "repository": REPOSITORY, "leftOut": true }),
        )
        .await;
    assert_eq!(refused["isError"], true, "{refused}");
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, vec!["Me".to_owned()]);
}

#[tokio::test]
async fn mcp_008_a_repository_description_comes_back_cleaned_and_cut() {
    let hub = hub().await;
    let hostile = format!("ok\u{1b}[2J\u{7}line\nbreak {}", "x".repeat(2_000));
    projects_answer(&hub, &hostile);
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session.call("anima_projects_search", json!({})).await;
    let shown = structured(&answer)["projects"][0]["repositories"][0]["description"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(shown.starts_with("ok [2J line break "), "{shown}");
    assert!(!shown.chars().any(|c| c.is_control()), "{shown:?}");
    assert!(shown.contains("…[cut: "), "{shown}");
    assert!(shown.chars().count() < 600, "{}", shown.chars().count());
    session.finish().await;
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
