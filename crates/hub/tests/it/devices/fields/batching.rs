//! API-005: `status`, `uptime`, `checks` and `ip` cost a constant number of statements however
//! many devices a list returns.

use std::sync::{Arc, atomic::Ordering};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        checks::graphql::{check_status_loader, check_uptime_loader, last_heartbeat_loader},
        devices::graphql::{
            Mutation, Query, checks_loader, resolved_ip_loader, status_loader, uptime_loader,
        },
    },
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::Value;
use sqlx::PgPool;

use super::{OLLAMA, seed_standing};
use crate::devices::{
    batching::{BATCH_WINDOW, SITES, counting_pool},
    graphql_read::add_bulk,
    harness::{TestSchema, data, now},
};

/// The batched cost of the fields below is 9 statements (the page, its count, and the reads of
/// the checks face); the in-memory path adds 3. A loaded machine can split batches, and more of
/// them on the large set (38 statements were seen there), so the bounds leave room for that. A
/// per-device read costs at least one statement per device, 163 on the large set, twice the
/// large-set bound.
const BATCHED_AT_MOST: usize = 30;
const IN_MEMORY_AT_MOST: usize = 36;
const LARGE_SET_AT_MOST: usize = 80;

const PER_CHECK_FIELDS: &str = "totalCount edges { node { name status ip uptime(window: DAY) \
                                checks { name status lastHeartbeat { at } uptime(window: DAY) } } }";

const FIELDS: &str = "totalCount edges { node { name status ip uptime(window: DAY) \
                      week: uptime(window: WEEK) checks { name } } }";

fn schema(pool: &PgPool) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(status_loader(pool.clone(), Arc::clone(&clock)).delay(BATCH_WINDOW))
        .data(uptime_loader(pool.clone(), Arc::clone(&clock)).delay(BATCH_WINDOW))
        .data(checks_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(resolved_ip_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(check_status_loader(pool.clone(), Arc::clone(&clock)).delay(BATCH_WINDOW))
        .data(last_heartbeat_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(check_uptime_loader(pool.clone(), Arc::clone(&clock)).delay(BATCH_WINDOW))
        .data(clock)
        .finish()
}

fn rendered(result: &Value) -> usize {
    result["devices"]["edges"].as_array().unwrap().len()
}

async fn cost_for_13_then_163(pool: PgPool, query: &str) -> (usize, usize, Value) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = schema(&counted);
    seed_standing(&pool).await;
    sqlx::query(
        "UPDATE checks.checks SET resolved_ip = '10.10.0.77', last_beat_at = now() \
         WHERE device_id = $1::text::uuid AND name = 'Ping'",
    )
    .bind(OLLAMA)
    .execute(&pool)
    .await
    .unwrap();

    let few = data(&schema, query).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, SITES, 150).await;
    let many = data(&schema, query).await;
    let many_statements = statements.load(Ordering::SeqCst);
    assert_eq!((rendered(&few), rendered(&many)), (13, 163));
    (few_statements, many_statements, few)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn api_005_status_uptime_checks_and_ip_cost_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let query = format!("{{ devices(first: 200) {{ {PER_CHECK_FIELDS} }} }}");
    let (few, many, shown) = cost_for_13_then_163(pool, &query).await;
    // The loaders ran on real rows: the page shows the seeded values.
    let ollama = shown["devices"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["node"]["name"] == "ollama")
        .unwrap();
    assert_eq!(ollama["node"]["status"], "UP");
    assert_eq!(ollama["node"]["ip"], "10.10.0.77");
    assert_eq!(ollama["node"]["uptime"], 1.0);
    assert_eq!(ollama["node"]["checks"].as_array().unwrap().len(), 2);
    assert!(
        few <= BATCHED_AT_MOST && many <= LARGE_SET_AT_MOST,
        "{few} statements for 13 devices, {many} for 163"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn api_005_a_status_sort_with_a_filter_costs_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let query = format!(
        "{{ devices(first: 200, sort: UPTIME, filter: {{ status: [UP, DOWN, UNKNOWN, DEGRADED, MAINTENANCE] }}) {{ {FIELDS} }} }}"
    );
    let (few, many, _) = cost_for_13_then_163(pool, &query).await;
    assert!(
        few <= IN_MEMORY_AT_MOST && many <= LARGE_SET_AT_MOST,
        "{few} statements for 13 devices, {many} for 163"
    );
}
