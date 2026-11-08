//! `checkDefaults` and `setCheckDefaults`: what they store, refuse, audit and hand to the
//! default check of a new device (D-26, DB-011, API-003, LOG-005).

use anima_hub::checks::{self, CheckSettings};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    harness::{PVE_01, data_with, error_json, schema_for, validation_fields, with_variables},
    writes::rows,
};

const READ: &str =
    "{ checkDefaults { intervalSeconds retries retryIntervalSeconds degradedThresholdMs } }";
const SET: &str = "mutation($input: CheckDefaultsInput!) { setCheckDefaults(input: $input) {
    intervalSeconds retries retryIntervalSeconds degradedThresholdMs } }";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn values(interval: i32, retries: i32, retry_interval: i32, degraded: i32) -> Value {
    json!({
        "intervalSeconds": interval, "retries": retries,
        "retryIntervalSeconds": retry_interval, "degradedThresholdMs": degraded,
    })
}

async fn stored(pool: &PgPool) -> (i32, i32, i32, i32) {
    sqlx::query_as(
        "SELECT interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms \
         FROM checks.defaults",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fixture_test! {
    async fn db_011_the_seeded_defaults_are_the_spec_values(pool: PgPool) {
        assert_eq!(stored(&pool).await, (60, 3, 20, 1000));
        let data = data_with(&schema_for(&pool), READ, json!({})).await;
        assert_eq!(data["checkDefaults"], values(60, 3, 20, 1000));
    }

    async fn db_011_set_check_defaults_stores_and_returns_the_values(pool: PgPool) {
        let schema = schema_for(&pool);
        let data = data_with(&schema, SET, json!({ "input": values(30, 1, 10, 500) })).await;
        assert_eq!(data["setCheckDefaults"], values(30, 1, 10, 500));
        assert_eq!(stored(&pool).await, (30, 1, 10, 500));
        let read = data_with(&schema, READ, json!({})).await;
        assert_eq!(read["checkDefaults"], values(30, 1, 10, 500));
    }

    async fn db_011_the_bounds_of_checks_are_the_bounds_of_the_defaults(pool: PgPool) {
        let schema = schema_for(&pool);
        let refused = [
            ("intervalSeconds", values(9, 3, 20, 1000)),
            ("intervalSeconds", values(86_401, 3, 20, 1000)),
            ("retries", values(60, 11, 20, 1000)),
            ("retryIntervalSeconds", values(60, 3, 4, 1000)),
            ("retryIntervalSeconds", values(60, 3, 3_601, 1000)),
            ("degradedThresholdMs", values(60, 3, 20, 0)),
            ("degradedThresholdMs", values(60, 3, 20, 600_001)),
            ("retries", values(60, -1, 20, 1000)),
        ];
        for (field, input) in refused {
            let response = with_variables(&schema, SET, json!({ "input": input })).await;
            let error = error_json(&response);
            assert_eq!(validation_fields(&error), [field], "{input}");
        }
        assert_eq!(stored(&pool).await, (60, 3, 20, 1000));
        assert!(rows(&pool, "audit_log").await.is_empty());
    }

    async fn db_011_a_value_in_range_at_each_bound_is_stored(pool: PgPool) {
        let schema = schema_for(&pool);
        for input in [values(10, 0, 5, 1), values(86_400, 10, 3_600, 600_000)] {
            data_with(&schema, SET, json!({ "input": input })).await;
        }
        assert_eq!(stored(&pool).await, (86_400, 10, 3_600, 600_000));
    }

    async fn log_005_set_check_defaults_writes_one_audit_entry_and_one_event(pool: PgPool) {
        let schema = schema_for(&pool);
        data_with(&schema, SET, json!({ "input": values(30, 1, 10, 500) })).await;
        let audits = rows(&pool, "audit_log").await;
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0]["action"], "update");
        assert_eq!(audits[0]["change"], values(30, 1, 10, 500));
        assert_eq!(rows(&pool, "event_log").await.len(), 1);
    }

    async fn d_26_a_check_added_after_set_check_defaults_takes_the_new_settings(pool: PgPool) {
        let schema = schema_for(&pool);
        data_with(&schema, SET, json!({ "input": values(30, 1, 10, 500) })).await;
        let mut tx = pool.begin().await.unwrap();
        let device = PVE_01.parse().unwrap();
        let id = checks::add_default_check(&mut tx, device).await.unwrap();
        tx.commit().await.unwrap();
        let added = checks::get(&pool, id).await.unwrap();
        let expected = CheckSettings {
            interval_seconds: 30,
            retries: 1,
            retry_interval_seconds: 10,
            degraded_threshold_ms: 500,
            invert: false,
        };
        assert_eq!(added.settings, expected);
        assert_eq!(checks::defaults(&pool).await.unwrap().settings(), expected);
    }
}
