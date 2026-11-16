//! Each link change leaves one audit entry and one event; the event does not hold the label
//! (LOG-004, LOG-005).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{link, signed_in, unlink};
use crate::hub::unauthenticated;

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM events.audit_log t WHERE target_type = 'record_link' \
         ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_each_change_has_one_entry_and_a_repeat_adds_none(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let ends = (
        "book:1",
        "lent_to",
        "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d",
    );
    let made = link(&hub, &token, ends, json!("Ada")).await;
    link(&hub, &token, ends, json!("Ada")).await;
    link(&hub, &token, ends, json!("Ada Lovelace")).await;
    unlink(&hub, &token, &made["data"]["linkRecords"]["id"]).await;
    let entries = audits(&pool).await;
    let actions: Vec<_> = entries
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, ["create", "update", "delete"], "{entries:?}");
    assert_eq!(entries[0]["target_id"], made["data"]["linkRecords"]["id"]);
    assert_eq!(entries[1]["change"]["label"], "Ada Lovelace");

    let messages: Vec<String> = sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE message LIKE 'Record link%' ORDER BY at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages[0],
        "Record link added: book:1 lent_to person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d."
    );
    assert!(messages.iter().all(|message| !message.contains("Ada")));
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_links_need_a_token(pool: PgPool) {
    let (hub, _token) = signed_in(&pool).await;
    let response = hub
        .post(
            None,
            "{ recordLinks(record: \"book:1\") { id } }",
            Value::Null,
        )
        .await;
    assert_eq!(crate::hub::error(&response), unauthenticated());
}
