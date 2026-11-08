//! `setRetention` leaves one audit entry and one event, in the transaction of the change
//! (LOG-004, LOG-005, HUB-005).

use anima_hub::events::Source;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{CLIENT_ADDR, OPERATOR_ID, SET, error_json, now, schema_for, with_variables};

async fn rows(pool: &PgPool, table: &str) -> Vec<Value> {
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM events.{table} t ORDER BY at, id"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_set_retention_writes_the_period_one_audit_entry_and_one_event(pool: PgPool) {
    let response = with_variables(
        &schema_for(&pool),
        SET,
        json!({ "subject": "EVENTS", "days": 120 }),
    )
    .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    let (events, audits) = (
        rows(&pool, "event_log").await,
        rows(&pool, "audit_log").await,
    );
    assert_eq!((events.len(), audits.len()), (1, 1));
    let (event, audit) = (&events[0], &audits[0]);
    assert_eq!(event["source"], Source::Hub.as_str());
    assert_eq!(event["severity"], "info");
    assert_eq!(event["device_id"], Value::Null);
    assert_eq!(
        event["message"],
        "Retention of events set to 120 days (was 90)."
    );
    assert_eq!(audit["operator_id"], OPERATOR_ID);
    assert_eq!(audit["client_addr"], CLIENT_ADDR);
    assert_eq!(audit["action"], "update");
    assert_eq!(audit["target_type"], "setting");
    assert_eq!(audit["target_id"], "retention.events");
    assert_eq!(
        audit["change"],
        json!({ "subject": "events", "days": 120, "previousDays": 90 })
    );
    for at in [&event["at"], &audit["at"]] {
        let stamped: chrono::DateTime<chrono::Utc> = at.as_str().unwrap().parse().unwrap();
        assert_eq!(stamped, now());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_refused_period_leaves_no_entry(pool: PgPool) {
    let response = with_variables(
        &schema_for(&pool),
        SET,
        json!({ "subject": "DELIVERIES", "days": 6 }),
    )
    .await;
    assert_eq!(error_json(&response)["extensions"]["code"], "VALIDATION");
    assert_eq!(
        (
            rows(&pool, "event_log").await.len(),
            rows(&pool, "audit_log").await.len()
        ),
        (0, 0)
    );
}
