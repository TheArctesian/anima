//! Every device mutation leaves one event and one audit entry, in its own transaction
//! (LOG-004, LOG-005, LOG-007).

use anima_hub::events::{Signal, Source};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, OPERATOR_ID, TickingClock, error_json, schema_for, schema_with_bus,
    schema_with_clock, with_variables,
};

const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const AT: &str = "2026-11-04T09:00:00+00:00";
const CREATE: &str = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { id } }";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
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
    let (mut events, mut audits) = (rows(pool, "event_log").await, rows(pool, "audit_log").await);
    assert_eq!((events.len(), audits.len()), (1, 1));
    (events.remove(0), audits.remove(0))
}

fn assert_who_and_when(event: &Value, audit: &Value) {
    assert_eq!(event["severity"], "info");
    assert_eq!(event["source"], Source::Device.as_str());
    assert_eq!(audit["operator_id"], OPERATOR_ID);
    assert_eq!(audit["operator_name"], "operator");
    assert_eq!(audit["client_addr"], CLIENT_ADDR);
    for at in [&event["at"], &audit["at"]] {
        let stamped: chrono::DateTime<chrono::Utc> = at.as_str().unwrap().parse().unwrap();
        assert_eq!(stamped.to_rfc3339(), AT);
    }
}

fn pve_02() -> Value {
    json!({ "input": { "siteId": HOME_SITE, "name": "pve-02", "host": "pve-02.home.lan",
        "ssh": { "enabled": true, "username": "operator", "auth": "PASSWORD" } } })
}

fixture_test! {
    async fn log_004_create_site_writes_one_event_and_one_audit_entry(pool: PgPool) {
        let vars = json!({ "name": "office-3", "subnet": "10.40.0.0/24" });
        let query = "mutation($name: String!, $subnet: Cidr!) { createSite(name: $name, subnet: $subnet) { id } }";
        let (event, audit) = traced(&pool, query, vars).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Site office-3 added.");
        assert_eq!((event["device_id"].clone(), event["device_name"].clone()), (Value::Null, Value::Null));
        assert_eq!((audit["action"].clone(), audit["target_type"].clone()), (json!("create"), json!("site")));
        assert_eq!(audit["change"], json!({ "name": "office-3", "subnet": "10.40.0.0/24" }));
    }

    async fn log_005_create_device_records_the_input_without_a_secret(pool: PgPool) {
        let (event, audit) = traced(&pool, CREATE, pve_02()).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Device pve-02 added.");
        assert_eq!(event["device_name"], "pve-02");
        assert_eq!(event["device_id"], audit["target_id"]);
        assert_eq!((audit["action"].clone(), audit["target_type"].clone()), (json!("create"), json!("device")));
        assert_eq!(audit["change"], json!({
            "siteId": HOME_SITE, "name": "pve-02", "host": "pve-02.home.lan", "kind": "server",
            "os": "other", "tags": [],
            "ssh": { "enabled": true, "port": 22, "username": "operator", "signIn": "password" },
        }));
    }

    async fn log_005_update_records_only_the_fields_that_were_sent(pool: PgPool) {
        let query = "mutation($id: ID!, $input: UpdateDeviceInput!) { updateDevice(id: $id, input: $input) { id } }";
        let vars = json!({ "id": PVE_01, "input": { "maintenance": true, "tags": ["edge"] } });
        let (event, audit) = traced(&pool, query, vars).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Device pve-01 updated.");
        assert_eq!((event["device_id"].clone(), audit["target_id"].clone()), (json!(PVE_01), json!(PVE_01)));
        assert_eq!(audit["action"], "update");
        assert_eq!(audit["change"], json!({ "maintenance": true, "tags": ["edge"] }));
    }

    async fn log_005_delete_records_the_name_host_and_site_and_the_entries_outlive_the_device(pool: PgPool) {
        let query = "mutation($id: ID!) { deleteDevice(id: $id) }";
        let (event, audit) = traced(&pool, query, json!({ "id": PVE_01 })).await;
        assert_who_and_when(&event, &audit);
        assert_eq!(event["message"], "Device pve-01 deleted.");
        assert_eq!((event["device_id"].clone(), event["device_name"].clone()), (json!(PVE_01), json!("pve-01")));
        assert_eq!(audit["action"], "delete");
        assert_eq!(audit["change"], json!({ "name": "pve-01", "host": "pve-01.home.lan", "siteId": HOME_SITE }));
    }

    async fn log_004_a_mutation_reads_the_clock_once_for_the_write_the_event_and_the_audit_entry(pool: PgPool) {
        let (schema, _) = schema_with_clock(&pool, std::sync::Arc::new(TickingClock::new()));
        let query = "mutation($id: ID!, $input: UpdateDeviceInput!) { updateDevice(id: $id, input: $input) { updatedAt } }";
        let vars = json!({ "id": PVE_01, "input": { "maintenance": true } });
        let response = with_variables(&schema, query, vars).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let updated = response.data.into_json().unwrap()["updateDevice"]["updatedAt"].clone();
        let (events, audits) = (rows(&pool, "event_log").await, rows(&pool, "audit_log").await);
        let at = |value: &Value| value.as_str().unwrap().parse::<chrono::DateTime<chrono::Utc>>().unwrap();
        assert_eq!(at(&updated), at(&events[0]["at"]));
        assert_eq!(at(&updated), at(&audits[0]["at"]));
    }

    async fn log_005_an_update_with_no_fields_returns_the_device_and_writes_nothing(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let query = "mutation($id: ID!) { updateDevice(id: $id, input: {}) { name maintenance updatedAt } }";
        let response = with_variables(&schema, query, json!({ "id": PVE_01 })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let device = &response.data.into_json().unwrap()["updateDevice"];
        assert_eq!(device["name"], "pve-01");
        assert!(device["updatedAt"].as_str().unwrap().starts_with("2026-11-01"), "{device}");
        assert!(rows(&pool, "event_log").await.is_empty());
        assert!(rows(&pool, "audit_log").await.is_empty());
        assert!(receiver.try_recv().is_err());
        let unknown = json!({ "id": "0190a000-0000-7000-8002-0000000000ff" });
        let error = error_json(&with_variables(&schema, query, unknown).await);
        assert_eq!(error["extensions"]["code"], "NOT_FOUND");
    }

    async fn log_010_the_bus_receives_the_event_of_a_mutation(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let response = with_variables(&schema, CREATE, pve_02()).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let Signal::Event(event) = receiver.try_recv().unwrap() else {
            panic!("expected an event");
        };
        assert_eq!(event.source, Source::Device);
        assert_eq!(event.message, "Device pve-02 added.");
        assert_eq!(event.device.unwrap().1, "pve-02");
        assert_eq!(event.at.to_rfc3339(), AT);
        assert!(receiver.try_recv().is_err());
    }

    async fn log_005_a_failed_audit_rolls_back_the_device_write_and_returns_internal_without_sql(pool: PgPool) {
        sqlx::query("DROP TABLE events.audit_log").execute(&pool).await.unwrap();
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let calls = [
            (CREATE, pve_02()),
            ("mutation($id: ID!, $input: UpdateDeviceInput!) { updateDevice(id: $id, input: $input) { id } }",
             json!({ "id": PVE_01, "input": { "name": "renamed" } })),
            ("mutation($id: ID!) { deleteDevice(id: $id) }", json!({ "id": PVE_01 })),
            ("mutation($name: String!, $subnet: Cidr!) { createSite(name: $name, subnet: $subnet) { id } }",
             json!({ "name": "office-3", "subnet": "10.40.0.0/24" })),
        ];
        for (query, variables) in calls {
            let error = error_json(&with_variables(&schema, query, variables).await);
            assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
            let text = error.to_string();
            for leak in ["audit_log", "SELECT", "INSERT", "relation", "events."] {
                assert!(!text.contains(leak), "{text}");
            }
        }
        let stored: (i64, i64, i64, String) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM devices.devices), (SELECT count(*) FROM devices.sites), \
                    (SELECT count(*) FROM events.event_log), \
                    (SELECT name FROM devices.devices WHERE id = $1::text::uuid)",
        )
        .bind(PVE_01)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, (13, 3, 0, "pve-01".to_owned()));
        // D-5: the audit row is written first, so no subscriber saw an event that was rolled back.
        assert!(receiver.try_recv().is_err());
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_010_a_mutation_without_its_bus_or_clock_is_internal_and_stores_nothing(pool: PgPool) {
    use anima_hub::devices::graphql::{Mutation, Query};
    use async_graphql::{EmptySubscription, Schema};

    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .finish();
    let error = error_json(&with_variables(&schema, CREATE, pve_02()).await);
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    let text = error.to_string();
    for leak in ["Bus", "Clock", "Data `", "does not exist"] {
        assert!(!text.contains(leak), "{text}");
    }
    let devices: i64 = sqlx::query_scalar("SELECT count(*) FROM devices.devices")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(devices, 13);
}
