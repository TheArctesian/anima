//! The daily jobs through the tick: partitions ahead and dropped, aggregates trimmed and
//! orphan heartbeats swept, once a day (DB-004, DB-005, DB-009, R-004).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{DOCKER_HOST, JELLYFIN, add, database_now, device, rng, scheduler, tcp};
use crate::{
    checks::{self, Beat, CheckState},
    events::Bus,
    ids::CheckId,
    status::Status,
};

async fn partition_exists(pool: &PgPool, day: DateTime<Utc>) -> bool {
    let name = format!("checks.heartbeats_{}", day.format("%Y%m%d"));
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// As the owner, creates the heartbeat partition of the day 40 days before `now`.
async fn old_partition(pool: &PgPool, now: DateTime<Utc>) -> DateTime<Utc> {
    let old = now - TimeDelta::days(40);
    let day = old.date_naive();
    let next = day.succ_opt().unwrap();
    sqlx::query(&format!(
        "CREATE TABLE checks.heartbeats_{} PARTITION OF checks.heartbeats \
         FOR VALUES FROM ('{day}T00:00:00Z') TO ('{next}T00:00:00Z')",
        day.format("%Y%m%d")
    ))
    .execute(pool)
    .await
    .unwrap();
    old
}

/// Records a beat for a check and deletes the check, leaving an orphan heartbeat.
async fn orphan(pool: &PgPool, now: DateTime<Utc>) -> CheckId {
    let id = add(pool, device(JELLYFIN), tcp(22), 2).await;
    let beat = Beat {
        at: now,
        status: Status::Up,
        response_time_ms: Some(1),
        message: None,
        state: CheckState::default(),
        resolved_ip: None,
        cert_not_after: None,
    };
    let mut conn = pool.acquire().await.unwrap();
    checks::record_beat(&mut conn, id, &beat).await.unwrap();
    checks::delete_check(&mut conn, id).await.unwrap();
    id
}

async fn count(pool: &PgPool, sql: &str, id: CheckId) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

const BEATS: &str = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";

tick_test! {
    async fn db_005_the_first_tick_creates_days_ahead_and_drops_old_days(pool: PgPool) {
        let now = database_now(&pool).await;
        let old = old_partition(&pool, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;

        assert!(!partition_exists(&pool, old).await);
        for days in 0..=7 {
            assert!(partition_exists(&pool, now + TimeDelta::days(days)).await, "day {days}");
        }
    }

    async fn db_009_the_first_tick_trims_aggregates_and_sweeps_orphans(pool: PgPool) {
        let now = database_now(&pool).await;
        let kept = add(&pool, device(DOCKER_HOST), tcp(23), 2).await;
        sqlx::query("INSERT INTO checks.uptime_minute (check_id, bucket) VALUES ($1, $2)")
            .bind(kept)
            .bind(now - TimeDelta::days(8))
            .execute(&pool)
            .await
            .unwrap();
        let gone = orphan(&pool, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;

        let minutes = "SELECT count(*) FROM checks.uptime_minute WHERE check_id = $1";
        assert_eq!(count(&pool, minutes, kept).await, 0);
        assert_eq!(count(&pool, BEATS, gone).await, 0);
    }

    async fn r_004_the_daily_jobs_run_again_only_after_24_hours(pool: PgPool) {
        let now = database_now(&pool).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));
        scheduler.tick(now, &mut rng()).await;
        let gone = orphan(&pool, now).await;

        scheduler.tick(now + TimeDelta::hours(23), &mut rng()).await;
        assert_eq!(count(&pool, BEATS, gone).await, 1);
        scheduler.tick(now + TimeDelta::hours(24), &mut rng()).await;
        assert_eq!(count(&pool, BEATS, gone).await, 0);
    }
}
