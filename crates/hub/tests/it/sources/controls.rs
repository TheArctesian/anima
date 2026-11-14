//! Pause, sync now and remove (SRC-004, SRC-011, ITG-004): a paused account refuses "Sync now"
//! with CONFLICT, and removing it can keep or forget what it imported.

use std::time::Duration;

use anima_hub::ids::SourceAccountId;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Kit, items, kit, set_projects};
use crate::hub::{error, start};

const PAUSE: &str = "mutation($id: ID!, $paused: Boolean!) { setSourceAccountPaused(id: $id, paused: $paused) \
    { health pausedBy nextSyncAt } }";
const SYNC: &str = "mutation($id: ID!) { syncSourceAccount(id: $id) { id } }";
const REMOVE: &str = "mutation($id: ID!, $delete: Boolean!) { removeSourceAccount(id: $id, deleteRecords: $delete) }";

async fn pause(kit: &Kit, id: SourceAccountId, paused: bool) -> Value {
    kit.hub
        .post(
            Some(&kit.token),
            PAUSE,
            json!({ "id": id, "paused": paused }),
        )
        .await
}

async fn actions(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT action FROM events.audit_log WHERE target_type = 'source_account' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Waits for the run a mutation started to end.
async fn ended(pool: &PgPool) {
    for _ in 0..200 {
        let open: i64 =
            sqlx::query_scalar("SELECT count(*) FROM sources.sync_runs WHERE ended_at IS NULL")
                .fetch_one(pool)
                .await
                .unwrap();
        let any: i64 = sqlx::query_scalar("SELECT count(*) FROM sources.sync_runs")
            .fetch_one(pool)
            .await
            .unwrap();
        if any > 0 && open == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the run did not end");
}

#[sqlx::test(migrations = "./migrations")]
async fn src_004_a_paused_account_refuses_sync_now_until_it_is_resumed(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1")]]);
    let paused = pause(&kit, id, true).await;
    assert_eq!(
        paused["data"]["setSourceAccountPaused"],
        json!({ "health": "PAUSED", "pausedBy": "OPERATOR", "nextSyncAt": null })
    );
    pause(&kit, id, true).await;
    let response = kit
        .hub
        .post(Some(&kit.token), SYNC, json!({ "id": id }))
        .await;
    assert_eq!(error(&response).1, "CONFLICT", "{response}");
    assert_eq!(kit.run(id).await, None, "a paused account is not claimed");
    assert_eq!(kit.stub.gets(), Vec::<String>::new());

    let resumed = pause(&kit, id, false).await;
    assert_eq!(
        resumed["data"]["setSourceAccountPaused"]["nextSyncAt"],
        json!(start().to_rfc3339())
    );
    assert_eq!(
        actions(&pool).await,
        ["create", "pause", "resume"],
        "a second pause changes nothing"
    );
    let response = kit
        .hub
        .post(Some(&kit.token), SYNC, json!({ "id": id }))
        .await;
    assert!(response["errors"].is_null(), "{response}");
    ended(&pool).await;
    assert_eq!(kit.stub.gets(), ["/pages/1"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn itg_004_removing_an_account_keeps_or_forgets_its_records(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1"), ("b", "1")]]);
    kit.run(id).await;
    let removed = kit
        .hub
        .post(
            Some(&kit.token),
            REMOVE,
            json!({ "id": id, "delete": false }),
        )
        .await;
    assert_eq!(
        removed["data"]["removeSourceAccount"],
        json!(id),
        "{removed}"
    );
    assert_eq!(items(&pool, id).await.len(), 2, "kept");
    let left: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM sources.accounts), (SELECT count(*) FROM sources.sync_runs), \
         (SELECT count(*) FROM credentials.source_secrets)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(left, (0, 0, 0));

    let (_, again) = kit.connect().await;
    kit.run(again).await;
    let removed = kit
        .hub
        .post(
            Some(&kit.token),
            REMOVE,
            json!({ "id": again, "delete": true }),
        )
        .await;
    assert!(removed["errors"].is_null(), "{removed}");
    assert_eq!(items(&pool, again).await, []);
    let change: Value = sqlx::query_scalar(
        "SELECT change FROM events.audit_log WHERE action = 'delete' AND target_id = $1",
    )
    .bind(again.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(change["deleteRecords"], true);
    assert_eq!(change["recordsDeleted"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn src_004_an_unknown_account_is_not_found(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = Uuid::now_v7();
    for (query, variables) in [
        (PAUSE, json!({ "id": id, "paused": true })),
        (SYNC, json!({ "id": id })),
        (REMOVE, json!({ "id": id, "delete": true })),
    ] {
        let response = kit.hub.post(Some(&kit.token), query, variables).await;
        assert_eq!(error(&response).1, "NOT_FOUND", "{response}");
    }
}
