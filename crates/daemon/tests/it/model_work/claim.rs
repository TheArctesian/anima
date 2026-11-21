use anima_daemon::hub::{HubClient, TIMEOUT};
use daemon_core::wire::DaemonToken;
use serde_json::json;

use super::kit::node;
use crate::fake_hub::{FakeHub, Reply};

fn token() -> DaemonToken {
    DaemonToken::from_random([7; 32])
}

fn item(work_type: &str, parameters: &serde_json::Value) -> String {
    json!({
        "id": "0192f6aa-0000-7000-8000-000000000009", "type": work_type, "kind": "model", "attempt": 1,
        "lease": { "until": "2026-11-20T10:01:30Z" },
        "limits": { "seconds": 600, "idleSeconds": 60, "memoryBytes": 1, "diskBytes": 1, "orphanSeconds": 60 },
        "project": null, "secrets": [], "parameters": parameters, "actor": "User",
    })
    .to_string()
}

#[tokio::test]
async fn the_claim_names_only_the_built_model_types() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let root = tempfile::tempdir().expect("a folder");
    let mut node = node(root.path());
    assert_eq!(node.cycle(&client, &token()).await, Ok(None));
    let claim = &hub.seen("claim")[0].body;
    assert_eq!(claim["kinds"], json!(["model"]));
    assert_eq!(
        claim["types"],
        json!([
            "model_report",
            "runtime_install",
            "server_start",
            "server_stop"
        ])
    );
    assert!(hub.seen("report").is_empty());
}

#[tokio::test]
async fn work_outside_the_list_is_refused() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let root = tempfile::tempdir().expect("a folder");
    let mut node = node(root.path());
    for (work_type, cause) in [
        ("build", "type_unknown"),
        ("shell", "type_unknown"),
        ("model_measure", "type_unknown"),
    ] {
        let parameters = json!({ "server": "0192f6aa-0000-7000-8000-000000000001", "copy": "0192f6aa-0000-7000-8000-000000000002", "probe": "text_short" });
        hub.queue_claim(Reply::json(200, &item(work_type, &parameters)));
        node.cycle(&client, &token()).await.expect("a cycle");
        let reports = hub.seen("report");
        assert!(
            reports
                .iter()
                .all(|report| report.body["state"] != json!("running"))
        );
        let report = &reports.last().expect("a report").body;
        assert_eq!(report["state"], json!("refused"), "{work_type}");
        assert_eq!(report["result"]["cause"], json!(cause), "{work_type}");
    }
}

#[tokio::test]
async fn a_model_report_gives_the_state_and_the_settings() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let root = tempfile::tempdir().expect("a folder");
    let mut node = node(root.path());
    hub.queue_claim(Reply::json(200, &item("model_report", &json!({}))));
    node.cycle(&client, &token()).await.expect("a cycle");
    let reports = hub.seen("report");
    assert_eq!(reports[0].body["state"], json!("running"));
    let model = &reports[1].body["result"]["model"];
    assert_eq!(reports[1].body["state"], json!("succeeded"));
    assert_eq!(model["settings"]["sandbox"], json!(false));
    assert_eq!(model["installed"], json!([]));
    assert_eq!(reports[1].bearer.as_deref(), Some(token().expose()));
}

#[tokio::test]
async fn an_owned_flag_and_an_agent_outside_agent_types_are_refused() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let root = tempfile::tempdir().expect("a folder");
    let mut node = node(root.path());
    let start = json!({ "server": "0192f6aa-0000-7000-8000-000000000001", "runtime": "llama_server", "version": "b8100", "backend": "cpu", "settings": { "host": "0.0.0.0" } });
    hub.queue_claim(Reply::json(200, &item("server_start", &start)));
    node.cycle(&client, &token()).await.expect("a cycle");
    assert_eq!(
        hub.seen("report")[0].body["result"]["cause"],
        json!("flag_owned")
    );
    node.settings.agent_types = vec![daemon_core::wire::WorkType::ServerStop];
    let mut agent: serde_json::Value =
        serde_json::from_str(&item("model_report", &json!({}))).expect("json");
    agent["actor"] = json!("Agent");
    hub.queue_claim(Reply::json(200, &agent.to_string()));
    node.cycle(&client, &token()).await.expect("a cycle");
    assert_eq!(
        hub.seen("report")[1].body["result"]["cause"],
        json!("actor_not_permitted")
    );
}
