//! Record links through the hub API (K-E4; LNK-001, LNK-002, LNK-004).

mod label;
mod subjects;
mod trail;

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const FIELDS: &str = "id source target kind label sourceTitle targetTitle createdAt";

/// A hub at `start()` with the fixture operator and a login token.
async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// `linkRecords` from `source` to `target` with `kind` and `label`.
async fn link(
    hub: &Hub,
    token: &str,
    (source, kind, target): (&str, &str, &str),
    label: Value,
) -> Value {
    let query = format!(
        "mutation($input: RecordLinkInput!) {{ linkRecords(input: $input) {{ {FIELDS} }} }}"
    );
    let input = json!({ "source": source, "target": target, "kind": kind, "label": label });
    hub.post(Some(token), &query, json!({ "input": input }))
        .await
}

async fn links_of(hub: &Hub, token: &str, record: &str) -> Value {
    let query =
        format!("query($record: String!) {{ recordLinks(record: $record) {{ {FIELDS} }} }}");
    hub.post(Some(token), &query, json!({ "record": record }))
        .await
}

async fn unlink(hub: &Hub, token: &str, id: &Value) -> Value {
    let query = "mutation($id: ID!) { unlinkRecords(id: $id) }";
    hub.post(Some(token), query, json!({ "id": id })).await
}

async fn stored(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM links.record_link")
        .fetch_one(pool)
        .await
        .unwrap()
}
