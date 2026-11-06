//! Every mutation leaves one audit entry and one event, in its own transaction
//! (LOG-004, LOG-005, LOG-007, WEB-009).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, DELETE, DISMISS, FINGERPRINT, HTTP_CHECK, OPERATOR_ID, PVE_01,
    accept_certificate_query, accept_suggestion_query, create, create_query, data, fixture_test,
    input, now, run, schema_for, update_query,
};

async fn audit_rows(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(t) FROM events.audit_log t ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn event_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM events.event_log WHERE source = 'device'")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The audit rows written since `seen`, which must be exactly one, with its who, where, when
/// and target checked.
async fn one_new(pool: &PgPool, seen: usize, action: &str) -> Value {
    let mut rows = audit_rows(pool).await.split_off(seen);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = rows.remove(0);
    assert_eq!(row["operator_id"], OPERATOR_ID);
    assert_eq!(row["operator_name"], "operator");
    assert_eq!(row["client_addr"], CLIENT_ADDR);
    assert_eq!(row["action"], action);
    assert_eq!(row["target_type"], "device");
    assert_eq!(row["target_id"], PVE_01);
    let at: chrono::DateTime<chrono::Utc> = row["at"].as_str().unwrap().parse().unwrap();
    assert_eq!(at, now());
    row
}

fn assert_no_secret(row: &Value) {
    let text = row.to_string();
    for forbidden in ["hunter2", "token=", "user:", "/private"] {
        assert!(!text.contains(forbidden), "{forbidden} in {text}");
    }
}

fixture_test! {
    async fn log_005_each_mutation_writes_one_audit_entry_and_one_event_without_userinfo_or_query(pool: PgPool) {
        let schema = schema_for(&pool);
        let url = "https://pve-01.home.lan:8006/private?token=abc";

        let id = create(&schema, url).await;
        let row = one_new(&pool, 0, "create").await;
        assert_eq!(row["change"]["interfaceId"], id.as_str());
        assert_eq!(row["change"]["origin"], "https://pve-01.home.lan:8006");
        assert_no_secret(&row);
        assert_eq!(event_count(&pool).await, 1);

        data(&schema, &update_query(), json!({ "id": id, "input": input(url) })).await;
        let row = one_new(&pool, 1, "update").await;
        assert_eq!(row["change"]["interfaceId"], id.as_str());
        assert_no_secret(&row);
        assert_eq!(event_count(&pool).await, 2);

        data(&schema, DELETE, json!({ "id": id })).await;
        let row = one_new(&pool, 2, "delete").await;
        assert_eq!(row["change"]["interfaceId"], id.as_str());
        assert_eq!(event_count(&pool).await, 3);
    }

    async fn web_009_accepting_a_certificate_audits_the_interface_origin_and_fingerprint(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan:8006/?token=abc").await;
        data(
            &schema,
            &accept_certificate_query(),
            json!({ "id": id, "fingerprint": FINGERPRINT }),
        )
        .await;
        let row = one_new(&pool, 1, "accept").await;
        assert_eq!(row["change"]["interfaceId"], id.as_str());
        assert_eq!(row["change"]["origin"], "https://pve-01.home.lan:8006");
        assert_eq!(row["change"]["fingerprint"], FINGERPRINT);
        assert_no_secret(&row);
        assert_eq!(event_count(&pool).await, 2);
    }

    async fn web_009_a_pin_dropped_by_an_origin_change_is_in_the_update_entry(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan/").await;
        data(&schema, &accept_certificate_query(), json!({ "id": id, "fingerprint": FINGERPRINT })).await;
        data(&schema, &update_query(), json!({ "id": id, "input": input("https://pve-01.home.lan:9443/") })).await;
        let row = one_new(&pool, 2, "update").await;
        assert_eq!(row["change"]["pinDropped"], true);
    }

    async fn web_002_suggestion_accept_and_dismiss_are_audited_on_the_device(pool: PgPool) {
        let schema = schema_for(&pool);
        data(&schema, &accept_suggestion_query(), json!({ "checkId": HTTP_CHECK })).await;
        let row = one_new(&pool, 0, "accept").await;
        assert_eq!(row["change"]["checkId"], HTTP_CHECK);
        assert_eq!(row["change"]["created"], true);
        assert_no_secret(&row);
        data(&schema, DISMISS, json!({ "checkId": HTTP_CHECK })).await;
        let row = one_new(&pool, 1, "update").await;
        assert_eq!(row["change"], json!({ "checkId": HTTP_CHECK, "dismissed": true }));
        assert_eq!(event_count(&pool).await, 2);
    }

    async fn log_005_a_refused_mutation_leaves_no_audit_entry_and_no_event(pool: PgPool) {
        let schema = schema_for(&pool);
        let response = run(
            &schema,
            &create_query(),
            json!({ "deviceId": PVE_01, "input": input("ftp://x/") }),
        )
        .await;
        assert_eq!(response.errors.len(), 1);
        assert_eq!(audit_rows(&pool).await.len(), 0);
        assert_eq!(event_count(&pool).await, 0);
    }
}
