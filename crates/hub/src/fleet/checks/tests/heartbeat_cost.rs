//! API-005: the heartbeat reads touch a page of rows per check, not the check's history.

use chrono::{DateTime, Days, NaiveTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::fleet::checks::store::{LATEST_SQL, PAGES_SQL};

const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const BEATS: i32 = 6000;
const LIMIT: i64 = 20;
const LIMIT_ROWS: f64 = 20.0;

/// When the newest beat is written: just after and just before midnight UTC, so one history
/// spans two days and the other one.
const NEWEST_AT: [&str; 2] = ["00:00:30", "23:59:30"];

/// `time` on the UTC day two days before the database's: in the past, and inside the window
/// `checks.ensure_day_partition` accepts, whatever the hour.
async fn two_days_ago_at(pool: &PgPool, time: &str) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    let day = now.date_naive() - Days::new(2);
    day.and_time(time.parse::<NaiveTime>().unwrap()).and_utc()
}

/// A check with `BEATS` heartbeats a second apart, the newest at `newest`, in the partitions
/// of every day they cover.
async fn check_with_history(pool: &PgPool, newest: DateTime<Utc>) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO checks.checks (id, device_id, type, name) \
         VALUES ($1, $2::uuid, 'icmp', $3)",
    )
    .bind(id)
    .bind(DEVICE)
    .bind(format!("Ping {newest}"))
    .execute(pool)
    .await
    .unwrap();
    let oldest = newest - TimeDelta::seconds(i64::from(BEATS - 1));
    for day in [oldest, newest] {
        sqlx::query("SELECT checks.ensure_day_partition($1)")
            .bind(day)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO checks.heartbeats (check_id, at, status, response_time_ms) \
         SELECT $1, $3 - make_interval(secs => n), 'up', 5 \
         FROM generate_series(0, $2 - 1) n",
    )
    .bind(id)
    .bind(BEATS)
    .bind(newest)
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
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_005_a_page_of_heartbeats_reads_the_page_not_the_history(pool: PgPool) {
    for time in NEWEST_AT {
        let id = check_with_history(&pool, two_days_ago_at(&pool, time).await).await;
        let read = rows_read(&plan_of(&pool, PAGES_SQL, id, Some(LIMIT)).await);
        assert!(
            read < 10.0 * LIMIT_ROWS,
            "read {read} rows for a page of {LIMIT} out of {BEATS}, newest at {time}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_005_the_latest_heartbeat_reads_a_few_rows_not_the_history(pool: PgPool) {
    for time in NEWEST_AT {
        let id = check_with_history(&pool, two_days_ago_at(&pool, time).await).await;
        let read = rows_read(&plan_of(&pool, LATEST_SQL, id, None).await);
        assert!(
            read < 50.0,
            "read {read} rows for the newest of {BEATS}, newest at {time}"
        );
    }
}
