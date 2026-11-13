//! A changed switch leaves one audit entry (LOG-004, EXT-011).

use serde_json::Value;
use sqlx::PgPool;

use super::{set, signed_in};

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM events.audit_log t WHERE target_type = 'setting' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_011_each_changed_switch_has_one_audit_entry_and_a_repeat_adds_none(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    set(&hub, &token, "projects", true).await;
    set(&hub, &token, "projects", true).await;
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["action"], "activate");
    assert_eq!(entries[0]["target_id"], "extensions.projects");

    set(&hub, &token, "projects", false).await;
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1]["action"], "deactivate");
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_011_a_refused_switch_leaves_no_entry(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    set(&hub, &token, "map", true).await;
    assert!(audits(&pool).await.is_empty());
}
