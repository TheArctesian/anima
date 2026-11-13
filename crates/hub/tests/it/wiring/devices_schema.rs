//! WP-06 H-2: the devices part of the hub schema, with its three loaders and the sessions
//! host-key loader, answers through `build_schema` (API-005).

use std::sync::atomic::Ordering;

use anima_hub::{fleet::sessions::Live, schema::build_schema};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{FINGERPRINT, PVE_01, accept_key, add_operator, data, hub_schema, wired};
use crate::devices::{batching::counting_pool, graphql_read::add_bulk};

/// Each device's name and its site's name.
fn device_sites(result: &Value) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = result["devices"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let node = &edge["node"];
            let name = |value: &Value| value.as_str().unwrap().to_owned();
            (name(&node["name"]), name(&node["site"]["name"]))
        })
        .collect();
    pairs.sort();
    pairs
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn h_2_devices_list_with_their_site(pool: PgPool) {
    let schema = hub_schema(&pool, &Live::default());
    let result = data(
        &schema,
        "{ devices(first: 50) { totalCount edges { node { name site { name } } } } }",
        json!({}),
    )
    .await;
    let expected: Vec<(String, String)> = sqlx::query_as(
        "select d.name, s.name from devices.devices d join devices.sites s on s.id = d.site_id \
         order by 1, 2",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(result["devices"]["totalCount"], 13);
    assert_eq!(device_sites(&result), expected);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn h_2_a_site_lists_its_devices(pool: PgPool) {
    let schema = hub_schema(&pool, &Live::default());
    let result = data(
        &schema,
        "{ sites { name devices { totalCount edges { node { name } } } } }",
        json!({}),
    )
    .await;
    let mut shown: Vec<(String, i64)> = result["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| {
            let devices = &site["devices"];
            let count = devices["edges"].as_array().unwrap().len();
            assert_eq!(devices["totalCount"], count, "{site}");
            (
                site["name"].as_str().unwrap().to_owned(),
                i64::try_from(count).unwrap(),
            )
        })
        .collect();
    shown.sort();
    let expected: Vec<(String, i64)> = sqlx::query_as(
        "select s.name, count(d.id) from devices.sites s \
         left join devices.devices d on d.site_id = s.id group by s.name order by 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(shown, expected);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn h_2_a_credential_set_through_the_hub_shows_as_stored(pool: PgPool) {
    add_operator(&pool).await;
    let schema = hub_schema(&pool, &Live::default());
    let set = "mutation($id: ID!) { setSshCredential(deviceId: $id, \
               credential: { password: \"a-throwaway-password\" }) { credentialStored } }";
    let stored = data(&schema, set, json!({ "id": PVE_01 })).await;
    let read = data(
        &schema,
        "{ devices(first: 50) { edges { node { id ssh { credentialStored } } } } }",
        json!({}),
    )
    .await;

    assert_eq!(stored["setSshCredential"]["credentialStored"], true);
    let flagged: Vec<&str> = read["devices"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| edge["node"]["ssh"]["credentialStored"] == true)
        .map(|edge| edge["node"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(flagged, vec![PVE_01]);
}

/// Every loader the hub registers, in one selection, both ways round.
const NESTED: &str = "{ \
    devices(first: 200) { edges { node { name site { name } \
        ssh { credentialStored hostKey { fingerprintSha256 } } } } } \
    sites { devices(first: 200) { edges { node { site { name } } } } } }";

/// Batched, 13 devices cost 8 statements: the page, its count, one batch each of sites,
/// secrets and host keys, the site list, one batch of their devices and one of those
/// devices' sites. Loading per key costs at least 3 per device, over 40.
const FEW_AT_MOST: usize = 12;

/// The hub's loaders wait 1 ms (the async-graphql default) for keys. In a debug build, 163
/// devices take longer than that to resolve, so a batch can split: 16 to 37 statements were
/// seen. Loading per key costs at least 3 per device, over 480; this bound is under half a
/// statement per device. `devices::batching` pins the constant cost with a 20 ms window.
const MANY_AT_MOST: usize = 80;

/// The fingerprints shown, and how many devices there are.
fn host_keys(result: &Value) -> (Vec<&str>, usize) {
    let edges = result["devices"]["edges"].as_array().unwrap();
    let keys = edges
        .iter()
        .filter_map(|edge| edge["node"]["ssh"]["hostKey"]["fingerprintSha256"].as_str())
        .collect();
    (keys, edges.len())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_the_hub_schema_batches_sites_devices_credentials_and_host_keys(pool: PgPool) {
    add_operator(&pool).await;
    accept_key(&pool, PVE_01, "operator").await;
    let (counted, statements) = counting_pool(&pool).await;
    let schema = build_schema(wired(&counted, &Live::default()));

    let few = data(&schema, NESTED, json!({})).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, 30, 150).await;
    let many = data(&schema, NESTED, json!({})).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!(host_keys(&few), (vec![FINGERPRINT], 13));
    assert_eq!(host_keys(&many), (vec![FINGERPRINT], 163));
    assert!(
        few_statements <= FEW_AT_MOST && many_statements <= MANY_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}
