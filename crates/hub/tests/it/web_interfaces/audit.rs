//! Every mutation leaves one audit entry and one event, in its own transaction
//! (LOG-004, LOG-005, LOG-007, WEB-009).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, DELETE, DISMISS, FINGERPRINT, HTTP_CHECK, OPERATOR_ID, PVE_01,
    accept_certificate_query, accept_suggestion_query, add_operator, create, create_query, data,
    fixture_test, input, now, run, schema_for, update_query,
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

/// From now on every audit insert raises, as a failing write after the domain write would.
async fn fail_audit_inserts(pool: &PgPool) {
    for statement in [
        "CREATE FUNCTION events.refuse_audit() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'audit refused'; END $$",
        "CREATE TRIGGER refuse_audit BEFORE INSERT ON events.audit_log \
         FOR EACH ROW EXECUTE FUNCTION events.refuse_audit()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }
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

    async fn cr_5_a_web_interface_event_names_the_device(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan:8006/").await;
        data(&schema, DELETE, json!({ "id": id })).await;
        let events: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT device_id::text, device_name FROM events.event_log \
             WHERE source = 'device' ORDER BY at, id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let named = (Some(PVE_01.to_owned()), Some("pve-01".to_owned()));
        assert_eq!(events, vec![named.clone(), named]);
    }

    async fn web_009_accepting_a_certificate_audits_the_interface_origin_and_fingerprint(pool: PgPool) {
        add_operator(&pool).await;
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
        add_operator(&pool).await;
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

    async fn log_005_an_update_whose_audit_entry_fails_changes_nothing(pool: PgPool) {
        add_operator(&pool).await;
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan/").await;
        data(&schema, &accept_certificate_query(), json!({ "id": id, "fingerprint": FINGERPRINT })).await;
        fail_audit_inserts(&pool).await;
        let moved = run(&schema, &update_query(), json!({ "id": id, "input": input("https://pve-01.home.lan:9443/") })).await;
        assert_eq!(moved.errors.len(), 1);
        let renew = run(&schema, &accept_certificate_query(), json!({ "id": id, "fingerprint": "ab".repeat(32) })).await;
        assert_eq!(renew.errors.len(), 1);
        let stored: (String, String) = sqlx::query_as(
            "SELECT i.url, p.fingerprint_sha256 FROM web_interfaces.interfaces i \
             JOIN web_interfaces.pins p ON p.interface_id = i.id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, ("https://pve-01.home.lan/".to_owned(), FINGERPRINT.to_owned()));
    }

    async fn web_009_the_pin_records_the_signed_in_operator_and_the_update_entry_the_previous_origin(pool: PgPool) {
        add_operator(&pool).await;
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan/").await;
        data(&schema, &accept_certificate_query(), json!({ "id": id, "fingerprint": FINGERPRINT })).await;
        let by: String = sqlx::query_scalar("SELECT accepted_by::text FROM web_interfaces.pins")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(by, OPERATOR_ID);
        data(&schema, &update_query(), json!({ "id": id, "input": { "label": "Admin", "url": "https://pve-01.home.lan:9443/" } })).await;
        let row = one_new(&pool, 2, "update").await;
        assert_eq!(row["change"]["previousOrigin"], "https://pve-01.home.lan");
        assert_eq!(row["change"]["previousLabel"], "Panel");
    }
}
