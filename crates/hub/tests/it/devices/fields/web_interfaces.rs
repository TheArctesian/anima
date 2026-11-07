//! `Device.webInterfaces` and `Device.webInterfaceSuggestions` (WEB-001, WEB-002, API-005).

use std::sync::{Arc, atomic::Ordering};

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query, web_interface_suggestions_loader, web_interfaces_loader},
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::Value;
use sqlx::PgPool;

use super::PVE_01;
use crate::devices::{
    batching::{BATCH_WINDOW, SITES, counting_pool},
    graphql_read::add_bulk,
    harness::{data, now, schema_for},
};

const HOME_ASSISTANT: &str = "0190a000-0000-7000-8002-000000000004";
const PIHOLE: &str = "0190a000-0000-7000-8002-000000000005";
const PVE_SUGGESTED_URL: &str = "https://pve-01.home.lan:8006/";

const READ: &str = "{ devices(first: 200) { edges { node { name webInterfaces { label url } \
                    webInterfaceSuggestions { label url } } } } }";

/// Adds an interface to a device; `seconds` after the test clock orders it among the others.
async fn add_interface(pool: &PgPool, device: &str, (label, url): (&str, &str), seconds: i64) {
    sqlx::query(
        "INSERT INTO web_interfaces.interfaces (id, device_id, label, url, open_in, created_at) \
         VALUES (gen_random_uuid(), $1::text::uuid, $2, $3, 'anima_tab', $4)",
    )
    .bind(device)
    .bind(label)
    .bind(url)
    .bind(now() + chrono::TimeDelta::seconds(seconds))
    .execute(pool)
    .await
    .unwrap();
}

fn node<'a>(result: &'a Value, name: &str) -> &'a Value {
    result["devices"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| &edge["node"])
        .find(|node| node["name"] == name)
        .unwrap()
}

fn urls(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["url"].as_str().unwrap())
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn web_001_and_002_a_device_reads_its_interfaces_in_creation_order_and_its_suggestions(
    pool: PgPool,
) {
    add_interface(&pool, PVE_01, ("Second", "http://b.home.lan/"), 2).await;
    add_interface(&pool, PVE_01, ("First", "http://a.home.lan/"), 1).await;
    add_interface(&pool, HOME_ASSISTANT, ("HA", "http://ha.home.lan:8123/"), 1).await;

    let result = data(&schema_for(&pool), READ).await;

    let pve = node(&result, "pve-01");
    assert_eq!(
        urls(&pve["webInterfaces"]),
        ["http://a.home.lan/", "http://b.home.lan/"]
    );
    assert_eq!(urls(&pve["webInterfaceSuggestions"]), [PVE_SUGGESTED_URL]);
    // Home Assistant already has its HTTP check's address as an interface: nothing to suggest.
    let home = node(&result, "home-assistant");
    assert_eq!(urls(&home["webInterfaces"]), ["http://ha.home.lan:8123/"]);
    assert_eq!(urls(&home["webInterfaceSuggestions"]), Vec::<&str>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn web_001_and_002_a_device_with_none_gets_empty_lists(pool: PgPool) {
    let result = data(&schema_for(&pool), READ).await;

    let device = node(&result, "pve-01");
    assert_eq!(device["webInterfaces"], serde_json::json!([]));
    let docker = node(&result, "docker-host");
    assert_eq!(docker["webInterfaceSuggestions"], serde_json::json!([]));
    assert_eq!(docker["webInterfaces"], serde_json::json!([]));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn api_005_interfaces_and_suggestions_cost_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .data(web_interfaces_loader(counted.clone()).delay(BATCH_WINDOW))
        .data(web_interface_suggestions_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();
    add_interface(&pool, PIHOLE, ("Pi-hole", "http://pi.home.lan/"), 1).await;

    let few = data(&schema, READ).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, SITES, 150).await;
    let many = data(&schema, READ).await;
    let many_statements = statements.load(Ordering::SeqCst);

    for result in [&few, &many] {
        assert_eq!(
            urls(&node(result, "pihole")["webInterfaces"]),
            ["http://pi.home.lan/"]
        );
        assert_eq!(
            urls(&node(result, "pve-01")["webInterfaceSuggestions"]),
            [PVE_SUGGESTED_URL]
        );
    }
    // The page, its count, then one batch of each field (the suggestions read interfaces and
    // checks too). A per-device read costs one statement per device and field, at least 26.
    assert!(
        few_statements <= 12 && many_statements <= 12,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}
