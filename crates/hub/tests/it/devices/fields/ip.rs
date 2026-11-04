//! `Device.ip`: the host when it is an IP literal, else the address the last check resolved.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{OLLAMA, PVE_01};
use crate::devices::{
    HOME_SITE,
    harness::{data, schema_for, with_variables},
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

/// Records that the "Ping" check of the device resolved `address`, as its latest beat.
async fn resolved(pool: &PgPool, device: &str, address: &str) {
    sqlx::query(
        "UPDATE checks.checks SET resolved_ip = $2::inet, last_beat_at = now() \
         WHERE device_id = $1::text::uuid AND name = 'Ping'",
    )
    .bind(device)
    .bind(address)
    .execute(pool)
    .await
    .unwrap();
}

async fn ip_of(pool: &PgPool, device: &str) -> Value {
    let query = format!(r#"{{ device(id: "{device}") {{ ip }} }}"#);
    data(&schema_for(pool), &query).await["device"]["ip"].clone()
}

async fn create(pool: &PgPool, name: &str, host: &str) -> String {
    let query = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { id ip } }";
    let input = json!({ "siteId": HOME_SITE, "name": name, "host": host });
    let response = with_variables(&schema_for(pool), query, json!({ "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["createDevice"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fixture_test! {
    async fn r_026_ip_is_the_stored_literal_even_when_a_check_resolved_another_address(pool: PgPool) {
        resolved(&pool, PVE_01, "10.10.0.99").await;
        assert_eq!(ip_of(&pool, PVE_01).await, "10.10.0.2");
    }

    async fn r_026_a_device_without_a_literal_shows_the_address_its_check_resolved(pool: PgPool) {
        assert_eq!(ip_of(&pool, OLLAMA).await, Value::Null);
        resolved(&pool, OLLAMA, "10.10.0.77").await;
        assert_eq!(ip_of(&pool, OLLAMA).await, "10.10.0.77");
    }

    async fn r_026_a_beat_with_no_address_leaves_ip_null(pool: PgPool) {
        sqlx::query("UPDATE checks.checks SET last_beat_at = now() WHERE device_id = $1::text::uuid")
            .bind(OLLAMA)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(ip_of(&pool, OLLAMA).await, Value::Null);
    }

    async fn r_026_a_new_device_with_a_literal_host_has_that_ip_and_a_named_host_gets_it_from_its_check(pool: PgPool) {
        let literal = create(&pool, "edge-1", "10.50.0.9").await;
        let named = create(&pool, "edge-2", "edge-2.home.lan").await;
        resolved(&pool, &literal, "10.50.0.200").await;
        assert_eq!(ip_of(&pool, &literal).await, "10.50.0.9");

        assert_eq!(ip_of(&pool, &named).await, Value::Null);
        resolved(&pool, &named, "10.50.0.10").await;
        assert_eq!(ip_of(&pool, &named).await, "10.50.0.10");
    }

    async fn r_026_ip_in_a_list_shows_each_devices_own_address(pool: PgPool) {
        resolved(&pool, OLLAMA, "10.10.0.77").await;
        let result = data(&schema_for(&pool), "{ devices(first: 200) { edges { node { name ip } } } }").await;
        let shown: Vec<(&str, Option<&str>)> = result["devices"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e["node"]["name"].as_str().unwrap(), e["node"]["ip"].as_str()))
            .collect();
        assert!(shown.contains(&("ollama", Some("10.10.0.77"))), "{shown:?}");
        assert!(shown.contains(&("pve-01", Some("10.10.0.2"))), "{shown:?}");
        assert!(shown.contains(&("pihole", None)), "{shown:?}");
    }
}
