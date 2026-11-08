//! D-24, API-005: one build costs the same eight statements for 28 checks as for 328, and a
//! cached scrape costs none. Statements are counted by the TCP proxy of the batching test.

use std::sync::atomic::Ordering;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{body, database_now, rig};
use crate::checks::face::batching::{add_bulk, counting_pool};

/// The bulk checks of the batching test, given the configuration their type needs.
async fn add_readable_bulk(pool: &PgPool) {
    add_bulk(pool).await;
    for (kind, config) in [
        ("icmp", r#"{"timeoutMs":2000,"ipVersion":"V4"}"#),
        ("tcp", r#"{"port":22,"timeoutMs":5000}"#),
    ] {
        sqlx::query(
            "UPDATE checks.checks SET config = $2::jsonb WHERE type = $1 AND config = '{}'",
        )
        .bind(kind)
        .bind(config)
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn checks_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM checks.checks")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(
        path = "../../../fixtures",
        scripts("devices", "checks", "maintenance")
    )
)]
async fn d_24_a_build_costs_eight_statements_for_28_or_328_checks(pool: PgPool) {
    let (counted, count) = counting_pool(&pool).await;
    let (router, clock) = rig(&counted, database_now(&pool).await);
    // The connection's own setup statements are not the build's.
    body(&router).await;

    clock.advance(TimeDelta::seconds(10));
    count.store(0, Ordering::SeqCst);
    let few = body(&router).await;
    let few_cost = count.swap(0, Ordering::SeqCst);

    let cached = body(&router).await;
    let cached_cost = count.swap(0, Ordering::SeqCst);

    add_readable_bulk(&pool).await;
    clock.advance(TimeDelta::seconds(10));
    let many = body(&router).await;
    let many_cost = count.swap(0, Ordering::SeqCst);

    assert_eq!(checks_count(&pool).await, 328);
    assert!(many.lines().count() > few.lines().count() + 600);
    assert_eq!(cached, few);
    assert_eq!(cached_cost, 0);
    // Devices, maintenance windows, standings, checks, newest heartbeats and the 1d, 7d and
    // 30d uptime sums.
    assert_eq!((few_cost, many_cost), (8, 8));
}
