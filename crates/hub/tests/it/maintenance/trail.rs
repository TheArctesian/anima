//! Every window mutation leaves one event and one audit entry, in its own transaction
//! (LOG-004, LOG-005, LOG-007).

use std::sync::Arc;

use anima_hub::events::{Signal, Source};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, DELETE, OPERATOR_ID, TickingClock, create, create_query, error_json, manual_input,
    schema_for, schema_with_clock, switch_query, update_query, weekday_input, with_variables,
};

const SUNDAY_UPDATES: &str = "0190a000-0000-7000-8004-000000000001";
const AT: &str = "2026-11-04T09:00:00+00:00";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

async fn rows(pool: &PgPool, table: &str) -> Vec<Value> {
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM events.{table} t ORDER BY at, id"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Runs the mutation, which must succeed, and returns the one event and the one audit row.
async fn traced(pool: &PgPool, query: &str, variables: Value) -> (Value, Value) {
    let response = with_variables(&schema_for(pool), query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    one_pair(pool).await
}

async fn one_pair(pool: &PgPool) -> (Value, Value) {
    let (mut events, mut audits) = (rows(pool, "event_log").await, rows(pool, "audit_log").await);
    assert_eq!((events.len(), audits.len()), (1, 1));
    (events.remove(0), audits.remove(0))
}

fn assert_who_and_when(event: &Value, audit: &Value) {
    assert_eq!(event["severity"], "info");
    assert_eq!(event["source"], Source::Hub.as_str());
    assert_eq!(event["device_id"], Value::Null);
    assert_eq!(audit["operator_id"], OPERATOR_ID);
    assert_eq!(audit["operator_name"], "operator");
    assert_eq!(audit["client_addr"], CLIENT_ADDR);
    assert_eq!(audit["target_type"], "maintenance_window");
    for at in [&event["at"], &audit["at"]] {
        let stamped: chrono::DateTime<chrono::Utc> = at.as_str().unwrap().parse().unwrap();
        assert_eq!(stamped.to_rfc3339(), AT);
    }
}

fixture_test! {
    async fn log_005_create_records_the_whole_window(pool: PgPool) {
        let (event, audit) = traced(&pool, &create_query(), json!({ "input": weekday_input("Patch") })).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Maintenance window Patch added.");
        assert_eq!(audit["action"], "create");
        assert_eq!(audit["change"], json!({
            "name": "Patch", "timezone": "Pacific/Auckland", "kind": "recurring_weekday",
            "weekdays": ["sun"], "startTime": "03:00:00", "durationMinutes": 60,
            "targets": [{ "type": "tag", "target": "infra" }],
        }));
    }

    async fn log_005_update_records_only_the_fields_that_changed(pool: PgPool) {
        let mut input = weekday_input("Sunday updates");
        input["durationMinutes"] = json!(90);
        input["targets"] = json!([{ "type": "TAG", "target": "infra" }, { "type": "TAG", "target": "lab" }]);
        let (event, audit) = traced(&pool, &update_query(), json!({ "id": SUNDAY_UPDATES, "input": input })).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Maintenance window Sunday updates updated.");
        assert_eq!(audit["action"], "update");
        assert_eq!(audit["target_id"], SUNDAY_UPDATES);
        assert_eq!(audit["change"], json!({
            "durationMinutes": 90,
            "targets": [{ "type": "tag", "target": "infra" }, { "type": "tag", "target": "lab" }],
        }));
    }

    async fn log_005_update_names_a_dropped_rule_field_as_null(pool: PgPool) {
        let input = json!({
            "name": "Sunday updates", "kind": "SINGLE", "timezone": "Pacific/Auckland",
            "startsAt": "2026-11-04T09:00:00Z", "endsAt": "2026-11-04T10:00:00Z",
            "targets": [{ "type": "TAG", "target": "infra" }],
        });
        let (_, audit) = traced(&pool, &update_query(), json!({ "id": SUNDAY_UPDATES, "input": input })).await;
        assert_eq!(audit["change"], json!({
            "kind": "single", "weekdays": null, "startTime": null, "durationMinutes": null,
            "startsAt": "2026-11-04T09:00:00Z", "endsAt": "2026-11-04T10:00:00Z",
        }));
    }

    async fn log_005_delete_records_the_name_and_kind_and_the_entries_outlive_the_window(pool: PgPool) {
        let (event, audit) = traced(&pool, DELETE, json!({ "id": SUNDAY_UPDATES })).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Maintenance window Sunday updates deleted.");
        assert_eq!(audit["action"], "delete");
        assert_eq!(audit["target_id"], SUNDAY_UPDATES);
        assert_eq!(audit["change"], json!({ "name": "Sunday updates", "kind": "recurring_weekday" }));
    }

    async fn log_005_switching_a_manual_window_is_an_activate_or_deactivate_entry(pool: PgPool) {
        let id = create(&schema_for(&pool), manual_input("Freeze")).await;
        sqlx::query("DELETE FROM events.audit_log").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM events.event_log").execute(&pool).await.unwrap();

        let (event, audit) = traced(&pool, &switch_query(), json!({ "id": id, "active": true })).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Maintenance window Freeze switched on.");
        assert_eq!(audit["action"], "activate");
        assert_eq!(audit["target_id"], id);
        assert_eq!(audit["change"], json!({ "activeManual": true }));

        sqlx::query("DELETE FROM events.audit_log").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM events.event_log").execute(&pool).await.unwrap();
        let (event, audit) = traced(&pool, &switch_query(), json!({ "id": id, "active": false })).await;
        assert_eq!(event["message"], "Maintenance window Freeze switched off.");
        assert_eq!(audit["action"], "deactivate");
        assert_eq!(audit["change"], json!({ "activeManual": false }));
    }

    async fn log_004_a_mutation_reads_the_clock_once_for_the_write_the_event_and_the_audit_entry(pool: PgPool) {
        let (schema, _) = schema_with_clock(&pool, Arc::new(TickingClock::new()));
        let query = "mutation($input: MaintenanceWindowInput!) { createMaintenanceWindow(input: $input) { id } }";
        let response = with_variables(&schema, query, json!({ "input": weekday_input("Patch") })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let (event, audit) = one_pair(&pool).await;
        let stored: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("SELECT created_at FROM maintenance.windows WHERE name = 'Patch'")
                .fetch_one(&pool).await.unwrap();
        // The sign-in check reads the ticking clock first, so the mutation sees a later tick.
        assert!(stored >= AT.parse::<chrono::DateTime<chrono::Utc>>().unwrap());
        assert_eq!(event["at"], audit["at"]);
        assert_eq!(event["at"].as_str().unwrap().parse::<chrono::DateTime<chrono::Utc>>().unwrap(), stored);
    }

    async fn log_010_the_bus_receives_the_event_of_a_mutation(pool: PgPool) {
        let (schema, bus) = super::harness::schema_at(&pool, super::harness::now());
        let mut receiver = bus.subscribe();
        let response = with_variables(&schema, &create_query(), json!({ "input": weekday_input("Patch") })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let Signal::Event(event) = receiver.try_recv().unwrap() else {
            panic!("expected an event");
        };
        assert_eq!(event.source, Source::Hub);
        assert_eq!(event.message, "Maintenance window Patch added.");
        assert!(event.device.is_none());
        assert_eq!(event.at.to_rfc3339(), AT);
        assert!(receiver.try_recv().is_err());
    }

    async fn log_005_a_refused_mutation_leaves_no_event_and_no_audit_entry(pool: PgPool) {
        let (schema, bus) = super::harness::schema_at(&pool, super::harness::now());
        let mut receiver = bus.subscribe();
        let mut bad = weekday_input("Patch");
        bad["timezone"] = json!("nowhere");
        for (query, vars) in [
            (create_query(), json!({ "input": bad })),
            (update_query(), json!({ "id": "0190a000-0000-7000-8004-0000000000ff", "input": weekday_input("Ghost") })),
            (DELETE.to_owned(), json!({ "id": "0190a000-0000-7000-8004-0000000000ff" })),
            (switch_query(), json!({ "id": SUNDAY_UPDATES, "active": true })),
        ] {
            let response = with_variables(&schema, &query, vars).await;
            assert_eq!(response.errors.len(), 1, "{query}");
        }
        assert_eq!((rows(&pool, "event_log").await.len(), rows(&pool, "audit_log").await.len()), (0, 0));
        assert!(receiver.try_recv().is_err());
    }

    async fn log_005_a_failed_audit_rolls_back_the_window_write_and_returns_internal_without_sql(pool: PgPool) {
        let id = {
            let schema = schema_for(&pool);
            create(&schema, manual_input("Freeze")).await
        };
        sqlx::query("DROP TABLE events.audit_log").execute(&pool).await.unwrap();
        let (schema, bus) = super::harness::schema_at(&pool, super::harness::now());
        let mut receiver = bus.subscribe();
        let calls = [
            (create_query(), json!({ "input": weekday_input("Patch") })),
            (update_query(), json!({ "id": SUNDAY_UPDATES, "input": weekday_input("Renamed") })),
            (DELETE.to_owned(), json!({ "id": SUNDAY_UPDATES })),
            (switch_query(), json!({ "id": id, "active": true })),
        ];
        for (query, variables) in calls {
            let error = error_json(&with_variables(&schema, &query, variables).await);
            assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
            let text = error.to_string();
            for leak in ["audit_log", "SELECT", "INSERT", "relation", "events."] {
                assert!(!text.contains(leak), "{text}");
            }
        }
        let stored: (i64, i64, i64, bool, String) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM maintenance.windows), \
                    (SELECT count(*) FROM maintenance.window_targets), \
                    (SELECT count(*) FROM events.event_log), \
                    (SELECT active_manual FROM maintenance.windows WHERE id = $1::text::uuid), \
                    (SELECT name FROM maintenance.windows WHERE id = $2::text::uuid)",
        )
        .bind(&id)
        .bind(SUNDAY_UPDATES)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, (3, 3, 1, false, "Sunday updates".to_owned()));
        // The audit entry is written first, so no subscriber saw an event that was rolled back.
        // (The one stored event is the earlier create of "Freeze".)
        assert!(receiver.try_recv().is_err());
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("maintenance"))
)]
async fn api_010_a_mutation_without_its_bus_or_clock_is_internal_and_stores_nothing(pool: PgPool) {
    use anima_hub::maintenance::graphql::{Mutation, Query};
    use async_graphql::{EmptySubscription, Schema};

    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .finish();
    let query = create_query();
    let vars = json!({ "input": weekday_input("Patch") });
    let error = error_json(&with_variables(&schema, &query, vars).await);
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    let text = error.to_string();
    for leak in ["Bus", "Clock", "Data `", "does not exist"] {
        assert!(!text.contains(leak), "{text}");
    }
    let windows: i64 = sqlx::query_scalar("SELECT count(*) FROM maintenance.windows")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(windows, 2);
}
