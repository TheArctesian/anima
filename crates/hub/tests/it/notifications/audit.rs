//! Each channel mutation leaves one audit entry and one event in its transaction, naming the
//! secret fields it set or cleared and never a secret or a whole URL (LOG-004, LOG-005,
//! LOG-007).

use anima_hub::events::Source;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, DELETE, OPERATOR_ID, create, create_query, data, holds_no_secret, run, schema_for,
    update, webhook,
};

async fn rows(pool: &PgPool, table: &str) -> Vec<Value> {
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM events.{table} t ORDER BY at, id"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_create_update_and_delete_each_write_one_entry_without_a_secret(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    let change = json!({ "name": "Ops hook", "settings": { "webhook": {
        "url": "https://hooks.example.test/s3cr3t-path/2", "headers": [] } } });
    let response = update(&schema, &created["id"], change).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    data(&schema, DELETE, json!({ "id": created["id"] })).await;

    let (events, audits) = (
        rows(&pool, "event_log").await,
        rows(&pool, "audit_log").await,
    );
    assert_eq!((events.len(), audits.len()), (3, 3));
    for (event, verb) in events.iter().zip(["added", "updated", "deleted"]) {
        assert_eq!(event["source"], Source::Notifications.as_str());
        assert_eq!(
            event["message"],
            format!("Notification channel Ops hook {verb}.")
        );
        assert_eq!(event["device_id"], Value::Null);
    }
    for (audit, action) in audits.iter().zip(["create", "update", "delete"]) {
        assert_eq!(audit["action"], action);
        assert_eq!(audit["target_type"], "channel");
        assert_eq!(audit["target_id"], created["id"]);
        assert_eq!(audit["operator_id"], OPERATOR_ID);
        assert_eq!(audit["client_addr"], CLIENT_ADDR);
    }
    holds_no_secret("the audit log", &Value::Array(audits.clone()).to_string());
    holds_no_secret("the event log", &Value::Array(events).to_string());

    let added = &audits[0]["change"];
    assert_eq!(added["name"], "Ops hook");
    assert_eq!(added["kind"], "webhook");
    assert_eq!(added["settings"]["origin"], "https://hooks.example.test");
    assert_eq!(added["settings"]["headerNames"], json!(["X-Api-Key"]));
    assert_eq!(added["sealedSet"], json!(["url", "headers"]));
    assert_eq!(
        audits[1]["change"],
        json!({ "settings": { "method": "post", "headerNames": [] },
                "sealedSet": ["url"], "sealedCleared": ["headers"] })
    );
    assert_eq!(
        audits[2]["change"],
        json!({ "name": "Ops hook", "kind": "webhook" })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_refused_channel_leaves_no_entry(pool: PgPool) {
    let schema = schema_for(&pool);
    let input = json!({ "name": "Ops", "settings": { "webhook": { "url": "http://x.test/" } } });
    let response = run(&schema, &create_query(), json!({ "input": input })).await;
    assert_eq!(response.errors.len(), 1);
    assert_eq!(rows(&pool, "audit_log").await, Vec::<Value>::new());
    assert_eq!(rows(&pool, "event_log").await, Vec::<Value>::new());
}
