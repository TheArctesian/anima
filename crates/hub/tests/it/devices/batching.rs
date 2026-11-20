//! API-005: a nested selection costs the same number of SQL statements however many devices it returns.
//!
//! The test counts statements at the wire: a TCP proxy between the hub's pool and PostgreSQL
//! counts the client's Bind and simple Query messages. That is exact, needs no database
//! privileges and does not wait for statistics to flush.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use sqlx::PgPool;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        devices::graphql::{
            Mutation, Query, credential_loader, devices_by_site_loader, site_loader,
        },
        events::Bus,
        trust::graphql::host_key_loader,
    },
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::{Value, json};

use super::{
    credential::{PVE_01, store_password},
    graphql_read::add_bulk,
    harness::{TestSchema, data, now, vault},
};

/// A pool on the same database whose statements are counted.
pub(crate) async fn counting_pool(hub: &PgPool) -> (PgPool, Arc<AtomicUsize>) {
    crate::transport::postgres::counting_pool(hub, 10).await
}

const BATCHED_AT_MOST: usize = 12;
pub(super) const SITES: i32 = 30;
pub(super) const BATCH_WINDOW: Duration = Duration::from_millis(20);
const NESTED: &str =
    "{ sites { name devices(first: 200) { totalCount edges { node { name site { name } } } } } }";

fn rendered_devices(result: &serde_json::Value) -> usize {
    result["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["devices"]["edges"].as_array().unwrap().len())
        .sum()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_nested_site_device_site_costs_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(test_clock())
        .data(site_loader(counted.clone()).delay(BATCH_WINDOW))
        .data(devices_by_site_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();

    let few = data(&schema, NESTED).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);

    add_bulk(&pool, SITES, 150).await;

    let many = data(&schema, NESTED).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!((rendered_devices(&few), rendered_devices(&many)), (13, 163));
    // The batched cost is 3: the sites, one batch of devices for all sites, one batch of sites
    // for all devices. A loader that queries once per key costs about one statement per site, so
    // at least 33 on this data. The test loaders wait 20 ms, not the default 1 ms, so a loaded
    // machine cannot split a batch; the bound sits far below the per-key cost so it does not
    // depend on timing.
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}

const STORED: &str = "{ devices(first: 200) { edges { node { name ssh { credentialStored } } } } }";

/// The names of the devices whose `credentialStored` is true, and how many devices there are.
fn stored_flags(result: &serde_json::Value) -> (Vec<&str>, usize) {
    let edges = result["devices"]["edges"].as_array().unwrap();
    let stored = edges
        .iter()
        .filter(|edge| edge["node"]["ssh"]["credentialStored"] == true)
        .map(|edge| edge["node"]["name"].as_str().unwrap())
        .collect();
    (stored, edges.len())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_credential_stored_costs_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(test_clock())
        .data(credential_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();
    store_password(&pool, PVE_01, "a-stored-password").await;

    let few = data(&schema, STORED).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);

    add_bulk(&pool, SITES, 150).await;

    let many = data(&schema, STORED).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!(stored_flags(&few), (vec!["pve-01"], 13));
    assert_eq!(stored_flags(&many), (vec!["pve-01"], 163));
    // The batched cost is 3: the page, its count, one batch of secrets for every device. A
    // per-device lookup costs one statement per device, at least 15 on this data.
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}

const HOST_KEYS: &str =
    "{ devices(first: 200) { edges { node { name ssh { hostKey { fingerprintSha256 } } } } } }";

/// An ed25519 key and its fingerprint (throwaway, from `ssh-keygen`; the private half deleted).
const KEY: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
const FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";

/// Stores `KEY` as PVE-01's accepted host key, accepted by a new operator.
async fn accept_key(pool: &PgPool) {
    sqlx::query(
        "with operator as (insert into access.operators (id, username, password_hash, created_at) \
             values (gen_random_uuid(), 'operator', '$argon2id$x', now()) returning id) \
         insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
             first_seen_at, accepted_at, accepted_by) \
         select $1::text::uuid, 'ssh-ed25519', $2, $3, now(), now(), id from operator",
    )
    .bind(PVE_01)
    .bind(FINGERPRINT)
    .bind(KEY)
    .execute(pool)
    .await
    .unwrap();
}

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
async fn api_005_host_key_costs_a_constant_few_statements_for_13_or_163_devices(pool: PgPool) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(test_clock())
        .data(host_key_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();
    accept_key(&pool).await;

    let few = data(&schema, HOST_KEYS).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, SITES, 150).await;
    let many = data(&schema, HOST_KEYS).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!(host_keys(&few), (vec![FINGERPRINT], 13));
    assert_eq!(host_keys(&many), (vec![FINGERPRINT], 163));
    // The page, its count, one batch of host keys for every device (WP-10 T3).
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}

/// The statements one signed-in request costs on the counted pool, and its data.
async fn cost(schema: &TestSchema, statements: &AtomicUsize, query: &str) -> (usize, Value) {
    statements.store(0, Ordering::SeqCst);
    let result = data(schema, query).await;
    (statements.load(Ordering::SeqCst), result)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_credential_stored_in_a_set_or_clear_response_costs_no_statement(pool: PgPool) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(test_clock())
        .data(credential_loader(counted.clone()))
        .data(vault())
        .data(Bus::new(4))
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .finish();
    let set = |fields: &str| {
        format!(
            r#"mutation {{ setSshCredential(deviceId: "{PVE_01}", credential: {{ password: "a-password" }}) {{ {fields} }} }}"#
        )
    };
    let clear = |fields: &str| {
        format!(r#"mutation {{ clearSshCredential(deviceId: "{PVE_01}") {{ {fields} }} }}"#)
    };
    // Each pair runs on the same state: both sets replace a stored secret, both clears find none.
    cost(&schema, &statements, &set("enabled")).await;
    let (set_plain, _) = cost(&schema, &statements, &set("enabled")).await;
    let (set_known, set_data) = cost(&schema, &statements, &set("credentialStored")).await;
    cost(&schema, &statements, &clear("enabled")).await;
    let (clear_plain, _) = cost(&schema, &statements, &clear("enabled")).await;
    let (clear_known, clear_data) = cost(&schema, &statements, &clear("credentialStored")).await;

    assert_eq!(
        set_data["setSshCredential"]["credentialStored"],
        json!(true)
    );
    assert_eq!(
        clear_data["clearSshCredential"]["credentialStored"],
        json!(false)
    );
    assert_eq!(
        (set_known, clear_known),
        (set_plain, clear_plain),
        "credentialStored added statements to the response"
    );
}

/// The clock the access guard reads to check the identity (API-003).
fn test_clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(now()))
}
