//! The status and uptime fields judge at the schema clock's time (CHK-006, CHK-011).

use std::sync::Arc;

use anima_hub::clock::{Clock, FixedClock};
use serde_json::Value;
use sqlx::PgPool;

use super::seed_standing;
use crate::devices::harness::{data, schema_with_clock};

fn at(text: &str) -> Arc<dyn Clock> {
    Arc::new(FixedClock(text.parse().unwrap()))
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(
        path = "../../../fixtures",
        scripts("devices", "checks", "maintenance")
    )
)]
async fn chk_006_status_is_judged_at_the_schema_clock_time_so_a_window_turns_down_into_maintenance(
    pool: PgPool,
) {
    seed_standing(&pool).await;
    let one =
        "{ devices(first: 200, filter: { name: \"truenas\" }) { edges { node { status } } } }";
    // 23:45 NZDT: the "Overnight backup" window covers the `storage` tag, and truenas is Down.
    let (inside, _) = schema_with_clock(&pool, at("2026-11-04T10:45:00Z"));
    let (outside, _) = schema_with_clock(&pool, at("2026-11-04T09:00:00Z"));
    assert_eq!(
        data(&inside, one).await["devices"]["edges"][0]["node"]["status"],
        "MAINTENANCE"
    );
    assert_eq!(
        data(&outside, one).await["devices"]["edges"][0]["node"]["status"],
        "DOWN"
    );
    // The status filter judges at the same time: jellyfin (manual flag) and truenas.
    let filtered = "{ devices(first: 200, filter: { status: [MAINTENANCE] }) { totalCount } }";
    assert_eq!(data(&inside, filtered).await["devices"]["totalCount"], 2);
    assert_eq!(data(&outside, filtered).await["devices"]["totalCount"], 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn chk_011_uptime_is_judged_at_the_schema_clock_time(pool: PgPool) {
    seed_standing(&pool).await;
    let one = "{ devices(first: 200, filter: { name: \"truenas\" }) { edges { node { uptime(window: DAY) } } } }";
    let (same_day, _) = schema_with_clock(&pool, at("2026-11-04T09:30:00Z"));
    let (days_later, _) = schema_with_clock(&pool, at("2026-11-07T09:00:00Z"));
    assert_eq!(
        data(&same_day, one).await["devices"]["edges"][0]["node"]["uptime"],
        0.25
    );
    assert_eq!(
        data(&days_later, one).await["devices"]["edges"][0]["node"]["uptime"],
        Value::Null
    );
}
