//! API-005: the heartbeat reads touch a page of rows per check, not the check's history.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::checks::store::{LATEST_SQL, PAGES_SQL};

const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const BEATS: i32 = 6000;
const LIMIT: i64 = 20;
const LIMIT_ROWS: f64 = 20.0;

async fn check_with_history(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO checks.checks (id, device_id, type, name) \
         VALUES ($1, $2::uuid, 'icmp', 'Ping')",
    )
    .bind(id)
    .bind(DEVICE)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("SELECT checks.ensure_day_partition(now())")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO checks.heartbeats (check_id, at, status, response_time_ms) \
         SELECT $1, now() - make_interval(secs => n), 'up', 5 \
         FROM generate_series(1, $2) n",
    )
    .bind(id)
    .bind(BEATS)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("ANALYZE checks.heartbeats")
        .execute(pool)
        .await
        .unwrap();
    id
}

/// The rows every node of the plan returned, times its loops, summed over the tree.
fn rows_read(plan: &Value) -> f64 {
    let own =
        plan["Actual Rows"].as_f64().unwrap_or(0.0) * plan["Actual Loops"].as_f64().unwrap_or(1.0);
    own + plan["Plans"]
        .as_array()
        .map_or(0.0, |children| children.iter().map(rows_read).sum())
}

async fn plan_of(pool: &PgPool, sql: &str, check: Uuid, page: Option<i64>) -> Value {
    let explain = format!("EXPLAIN (ANALYZE, FORMAT JSON) {sql}");
    let query = sqlx::query_as::<_, (Value,)>(&explain).bind(vec![check]);
    let query = match page {
        Some(limit) => query.bind(None::<DateTime<Utc>>).bind(limit),
        None => query,
    };
    let (plan,) = query.fetch_one(pool).await.unwrap();
    plan[0]["Plan"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn api_005_a_page_of_heartbeats_reads_the_page_not_the_history(pool: PgPool) {
    let id = check_with_history(&pool).await;
    let read = rows_read(&plan_of(&pool, PAGES_SQL, id, Some(LIMIT)).await);
    assert!(
        read < 10.0 * LIMIT_ROWS,
        "read {read} rows for a page of {LIMIT} out of {BEATS}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn api_005_the_latest_heartbeat_reads_a_few_rows_not_the_history(pool: PgPool) {
    let id = check_with_history(&pool).await;
    let read = rows_read(&plan_of(&pool, LATEST_SQL, id, None).await);
    assert!(read < 50.0, "read {read} rows for the newest of {BEATS}");
}
