//! The session limit and the recording flag of a device through GraphQL: create, update, read
//! back and the refused values (RA-036, RA-040).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CREATE, HOME, PVE_01};
use crate::devices::harness::{error_json, schema_for, validation_fields, with_variables};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const SHOWN: &str = "{ id sessionLimit recordSessions }";
const UPDATE: &str = "mutation($id: ID!, $input: UpdateDeviceInput!) { updateDevice(id: $id, input: $input) { sessionLimit recordSessions } }";

async fn create(pool: &PgPool, extra: Value) -> Value {
    let mut input = json!({ "siteId": HOME, "name": "win-01", "host": "10.10.0.60" });
    input.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let query = CREATE.replace("{ id }", SHOWN);
    let response = with_variables(&schema_for(pool), &query, json!({ "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["createDevice"].take()
}

async fn update(pool: &PgPool, input: Value) -> Value {
    let response = with_variables(&schema_for(pool), UPDATE, json!({ "id": PVE_01, "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["updateDevice"].take()
}

async fn read(pool: &PgPool, id: &str) -> Value {
    let query = "query($id: ID!) { device(id: $id) { sessionLimit recordSessions } }";
    let response = with_variables(&schema_for(pool), query, json!({ "id": id })).await;
    response.data.into_json().unwrap()["device"].take()
}

async fn refused(pool: &PgPool, mutation: &str, vars: Value) -> Vec<String> {
    let response = with_variables(&schema_for(pool), mutation, vars).await;
    validation_fields(&error_json(&response)).into_iter().map(str::to_owned).collect()
}

fixture_test! {
    async fn ra_036_a_device_created_without_a_limit_has_none_and_does_not_record(pool: PgPool) {
        let mut created = create(&pool, json!({})).await;
        let id = created.as_object_mut().unwrap().remove("id").unwrap();
        let expected = json!({ "sessionLimit": null, "recordSessions": false });
        assert_eq!(created, expected);
        assert_eq!(read(&pool, id.as_str().unwrap()).await, expected);
    }

    async fn ra_036_a_device_created_with_a_limit_and_the_flag_reads_them_back(pool: PgPool) {
        let mut created = create(&pool, json!({ "sessionLimit": 100, "recordSessions": true })).await;
        let id = created.as_object_mut().unwrap().remove("id").unwrap();
        let expected = json!({ "sessionLimit": 100, "recordSessions": true });
        assert_eq!(created, expected);
        assert_eq!(read(&pool, id.as_str().unwrap()).await, expected);
    }

    async fn ra_036_create_accepts_the_bounds_1_and_100(pool: PgPool) {
        assert_eq!(create(&pool, json!({ "sessionLimit": 1 })).await["sessionLimit"], 1);
        let second = json!({ "name": "win-02", "host": "10.10.0.61", "sessionLimit": 100 });
        assert_eq!(create(&pool, second).await["sessionLimit"], 100);
    }

    async fn ra_036_create_refuses_a_limit_outside_1_to_100_on_the_session_limit_field(pool: PgPool) {
        for limit in [0, 101, -1, 256, i32::MAX] {
            let input = json!({ "siteId": HOME, "name": "win-01", "host": "10.10.0.60", "sessionLimit": limit });
            let fields = refused(&pool, CREATE, json!({ "input": input })).await;
            assert_eq!(fields, ["sessionLimit"], "{limit}");
        }
        let named: i64 = sqlx::query_scalar("SELECT count(*) FROM devices.devices WHERE name = 'win-01'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(named, 0);
    }

    async fn ra_036_update_sets_keeps_and_clears_the_limit(pool: PgPool) {
        assert_eq!(update(&pool, json!({ "sessionLimit": 5 })).await["sessionLimit"], 5);
        assert_eq!(update(&pool, json!({ "name": "pve-renamed" })).await["sessionLimit"], 5);
        assert_eq!(update(&pool, json!({ "sessionLimit": null })).await["sessionLimit"], 5);
        assert_eq!(update(&pool, json!({ "sessionLimit": 0 })).await["sessionLimit"], Value::Null);
        assert_eq!(read(&pool, PVE_01).await["sessionLimit"], Value::Null);
    }

    async fn ra_036_update_refuses_a_limit_outside_0_to_100(pool: PgPool) {
        update(&pool, json!({ "sessionLimit": 7 })).await;
        for limit in [101, -1] {
            let vars = json!({ "id": PVE_01, "input": { "sessionLimit": limit } });
            assert_eq!(refused(&pool, UPDATE, vars).await, ["sessionLimit"], "{limit}");
        }
        assert_eq!(read(&pool, PVE_01).await["sessionLimit"], 7);
    }

    async fn ra_040_the_recording_flag_is_set_and_kept_by_update(pool: PgPool) {
        assert_eq!(read(&pool, PVE_01).await["recordSessions"], false);
        assert_eq!(update(&pool, json!({ "recordSessions": true })).await["recordSessions"], true);
        assert_eq!(update(&pool, json!({ "recordSessions": null })).await["recordSessions"], true);
        assert_eq!(update(&pool, json!({ "sessionLimit": 2 })).await["recordSessions"], true);
        assert_eq!(update(&pool, json!({ "recordSessions": false })).await["recordSessions"], false);
    }
}
