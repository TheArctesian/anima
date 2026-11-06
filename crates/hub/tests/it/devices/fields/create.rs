//! `createDevice(addDefaultCheck)`: the default ICMP check is written in the device's own
//! transaction (spec 6.2, DEV-010, LOG-005).

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::devices::{
    HOME_SITE,
    harness::{error_json, only_error, schema_for, validation_fields, with_variables},
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const CREATE: &str = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) {
    id name status checks { name type intervalSeconds status } } }";

fn input(name: &str) -> Value {
    json!({ "siteId": HOME_SITE, "name": name, "host": format!("{name}.home.lan") })
}

async fn created(pool: &PgPool, input: Value) -> Value {
    let response = with_variables(&schema_for(pool), CREATE, json!({ "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["createDevice"].clone()
}

async fn stored_checks(pool: &PgPool, name: &str) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT c.name, c.type FROM checks.checks c JOIN devices.devices d ON d.id = c.device_id \
         WHERE d.name = $1 ORDER BY c.name",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn audit_change(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT change FROM events.audit_log WHERE target_type = 'device'")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn ping() -> Vec<(String, String)> {
    vec![("Ping".to_owned(), "icmp".to_owned())]
}

fixture_test! {
    async fn dev_010_add_default_check_defaults_to_one_icmp_ping_check(pool: PgPool) {
        let device = created(&pool, input("edge-1")).await;
        assert_eq!(stored_checks(&pool, "edge-1").await, ping());
        assert_eq!(
            device["checks"],
            json!([{ "name": "Ping", "type": "ICMP", "intervalSeconds": 60, "status": "UNKNOWN" }])
        );
        assert_eq!(device["status"], "UNKNOWN");
    }

    async fn dev_010_add_default_check_true_adds_the_check_and_records_it_in_the_audit_change(pool: PgPool) {
        let mut form = input("edge-1");
        form["addDefaultCheck"] = json!(true);
        created(&pool, form).await;
        assert_eq!(stored_checks(&pool, "edge-1").await, ping());
        assert_eq!(audit_change(&pool).await["defaultCheck"], true);
    }

    async fn dev_010_add_default_check_false_adds_no_check_and_the_audit_change_says_nothing_of_it(pool: PgPool) {
        let mut form = input("edge-1");
        form["addDefaultCheck"] = json!(false);
        let device = created(&pool, form).await;
        assert_eq!(stored_checks(&pool, "edge-1").await, Vec::new());
        assert_eq!(device["checks"], json!([]));
        assert!(audit_change(&pool).await.get("defaultCheck").is_none());
    }

    async fn dev_010_a_null_add_default_check_means_the_default(pool: PgPool) {
        let mut form = input("edge-1");
        form["addDefaultCheck"] = Value::Null;
        created(&pool, form).await;
        assert_eq!(stored_checks(&pool, "edge-1").await, ping());
    }

    async fn dev_010_an_invalid_device_adds_neither_the_device_nor_a_check(pool: PgPool) {
        let mut form = input("edge-1");
        form["host"] = json!("http://edge-1");
        let error = only_error_for(&pool, form).await;
        assert_eq!(validation_fields(&error), ["host"]);
        assert_eq!(stored_checks(&pool, "edge-1").await, Vec::new());
        assert_eq!(check_count(&pool).await, 28);
    }

    async fn log_005_a_failed_audit_rolls_back_the_device_and_its_default_check(pool: PgPool) {
        sqlx::query("DROP TABLE events.audit_log").execute(&pool).await.unwrap();
        let response = with_variables(&schema_for(&pool), CREATE, json!({ "input": input("edge-1") })).await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "INTERNAL");
        assert_eq!(device_count(&pool).await, 13);
        assert_eq!(check_count(&pool).await, 28);
    }

    async fn dev_010_a_failed_default_check_rolls_back_the_device_and_leaks_no_sql(pool: PgPool) {
        sqlx::query("DROP TABLE checks.checks CASCADE").execute(&pool).await.unwrap();
        let response = with_variables(&schema_for(&pool), CREATE, json!({ "input": input("edge-1") })).await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
        let text = error.to_string();
        for leak in ["checks.checks", "INSERT", "relation"] {
            assert!(!text.contains(leak), "{text}");
        }
        assert_eq!(device_count(&pool).await, 13);
        let events: i64 = sqlx::query_scalar("SELECT count(*) FROM events.audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(events, 0);
    }

    async fn dev_010_without_the_default_check_a_dropped_checks_table_does_not_matter(pool: PgPool) {
        sqlx::query("DROP TABLE checks.checks CASCADE").execute(&pool).await.unwrap();
        let mut form = input("edge-1");
        form["addDefaultCheck"] = json!(false);
        let response = with_variables(&schema_for(&pool), "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { name } }", json!({ "input": form })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(device_count(&pool).await, 14);
    }
}

async fn only_error_for(pool: &PgPool, form: Value) -> Value {
    let query = format!(
        "mutation {{ createDevice(input: {{ siteId: \"{HOME_SITE}\", name: {}, host: {} }}) {{ id }} }}",
        form["name"], form["host"]
    );
    only_error(&schema_for(pool), &query).await
}

async fn device_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM devices.devices")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn check_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM checks.checks")
        .fetch_one(pool)
        .await
        .unwrap()
}
