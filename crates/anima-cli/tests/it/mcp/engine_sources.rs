//! The tools of the `sources` set against the scripted hub (MCP-007, MCP-017).

use serde_json::{Value, json};

use super::engine::{sent, structured, writing};
use super::session::{Desk, LOGIN_TOKEN, hub};
use crate::kit::fake_hub::{FakeHub, data};

const ACCOUNT: &str = "0190a000-0000-7000-9001-000000000001";

fn account(paused: bool) -> Value {
    json!({ "id": ACCOUNT, "sourceKey": "github", "label": "home-app", "instanceKey": "1/2",
            "credentialStored": true, "health": if paused { "PAUSED" } else { "OK" },
            "pausedBy": if paused { json!("OPERATOR") } else { Value::Null },
            "lastSyncAt": null, "lastError": null, "failureCount": 0, "nextSyncAt": null,
            "syncing": false, "createdAt": "2026-11-12T10:00:00Z" })
}

fn accounts_answer(hub: &FakeHub) {
    let run = json!({ "id": "run-1", "trigger": "MANUAL", "startedAt": "2026-11-12T10:00:00Z",
                      "endedAt": null, "outcome": "OK", "inserted": 1, "changed": 0,
                      "unchanged": 2, "removed": 0, "problems": [], "error": null });
    let mut row = account(false);
    row["runs"] = json!([run]);
    hub.answer("SourceAccounts", data(json!({ "sourceAccounts": [row] })));
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
    assert_eq!(sent(&hub, "SourceRemove").len(), 0);
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
