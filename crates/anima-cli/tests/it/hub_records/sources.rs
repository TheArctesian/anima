//! `anima source kinds`, `list`, `get`, `sync`, `pause`, `resume` and `watch` against the
//! scripted hub (ITG-001 to ITG-003).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::subscription_hub::{Script, SubscriptionHub};

pub const ID: &str = "0190a000-0000-7000-8070-000000000001";

pub fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-source-tests")
}

pub fn account(health: &str) -> Value {
    json!({
        "id": ID, "sourceKey": "github", "label": "acme", "instanceKey": "123/456",
        "credentialStored": true, "health": health,
        "pausedBy": if health == "PAUSED" { json!("OPERATOR") } else { Value::Null },
        "lastSyncAt": "2026-11-12T08:00:00Z", "lastError": null, "failureCount": 0,
        "nextSyncAt": "2026-11-12T08:05:00Z", "syncing": false,
        "createdAt": "2026-11-01T08:00:00Z",
    })
}

/// The account as `SourceAccounts` returns it, with no runs.
pub fn listed(health: &str) -> Value {
    let mut account = account(health);
    account["runs"] = json!([]);
    account
}

fn with_runs(health: &str) -> Value {
    let mut account = account(health);
    account["runs"] = json!([{
        "id": "run-1", "trigger": "MANUAL", "startedAt": "2026-11-12T08:00:00Z",
        "endedAt": "2026-11-12T08:00:09Z", "outcome": "OK", "inserted": 3, "changed": 1,
        "unchanged": 10, "removed": 0,
        "problems": [{ "code": "ARCHIVED", "subject": "acme/old", "message": "skipped" }],
        "error": null,
    }]);
    account
}

#[tokio::test]
async fn itg_001_source_kinds_prints_the_k1_value_with_json() {
    let source = json!({
        "key": "github", "name": "GitHub", "acquisition": "TOKEN_PULL", "scopes": ["repo:read"],
        "writeReason": null, "gives": ["repositories"], "pollIntervalSeconds": 300,
        "consumers": ["projects"],
    });
    let hub = FakeHub::start().await;
    hub.answer("SourceKinds", data(json!({ "sources": [source] })));
    let run = signed_in()
        .run(&["source", "kinds", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), json!([source]));
    let run = signed_in()
        .run(&["source", "kinds", "--hub", &hub.origin])
        .await;
    assert!(
        run.stdout.contains("GitHub") && run.stdout.contains("300s"),
        "{run:?}"
    );
}

#[tokio::test]
async fn itg_003_source_list_and_get_print_the_k1_value_with_json() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [with_runs("OK")] })),
    );
    let run = signed_in()
        .run(&[
            "source",
            "list",
            "--source",
            "github",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), json!([with_runs("OK")]));
    assert_eq!(hub.seen()[0].variables["sourceKey"], "github");

    let run = signed_in()
        .run(&["source", "get", "acme", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), with_runs("OK"));
    assert_eq!(hub.seen().last().expect("sent").variables["runs"], 20);

    let run = signed_in()
        .run(&["source", "get", "acme", "--hub", &hub.origin])
        .await;
    assert!(
        run.stdout.contains("+3 ~1 =10 -0") && run.stdout.contains("acme/old"),
        "{run:?}"
    );
    let run = signed_in()
        .run(&["source", "get", "nobody", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn itg_002_source_sync_pause_and_resume_act_on_the_account_by_label() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [listed("OK")] })),
    );
    hub.answer(
        "SourceSync",
        data(json!({ "syncSourceAccount": account("OK") })),
    );
    hub.answer(
        "SourcePause",
        data(json!({ "setSourceAccountPaused": account("PAUSED") })),
    );
    let run = signed_in()
        .run(&["source", "sync", "acme", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), account("OK"));
    assert_eq!(hub.seen().last().expect("sent").variables["id"], ID);

    let run = signed_in()
        .run(&["source", "pause", "acme", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["health"], "PAUSED");
    assert_eq!(hub.seen().last().expect("sent").variables["paused"], true);

    let run = signed_in()
        .run(&["source", "resume", ID, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(hub.seen().last().expect("sent").variables["paused"], false);
}

#[tokio::test]
async fn itg_002_source_sync_of_a_paused_account_shows_the_hubs_conflict() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [listed("PAUSED")] })),
    );
    hub.answer("SourceSync", refusal("CONFLICT", "the account is paused"));
    let run = signed_in()
        .run(&["source", "sync", "acme", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("paused"), "{}", run.stderr);
}

#[tokio::test]
async fn cmd_020_source_watch_prints_one_line_for_each_change() {
    let change = json!({ "sourceAccountChanged": account("STALLED") });
    let hub = SubscriptionHub::start(Script::Next(vec![change])).await;
    let run = signed_in()
        .run(&["source", "watch", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("acme") && run.stdout.contains("STALLED"),
        "{run:?}"
    );
    assert_eq!(hub.received().subscribe["variables"]["id"], Value::Null);

    let change = json!({ "sourceAccountChanged": account("OK") });
    let hub = SubscriptionHub::start(Script::Next(vec![change])).await;
    let run = signed_in()
        .run(&["source", "watch", "--json", "--hub", &hub.origin])
        .await;
    let line: Value = serde_json::from_str(run.stdout.trim()).expect("JSON");
    assert_eq!(line, account("OK"));
}
