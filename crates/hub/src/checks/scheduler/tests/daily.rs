//! The daily jobs through the tick: partitions ahead and dropped, aggregates trimmed and
//! orphan heartbeats swept, once a day (DB-004, DB-005, DB-009).

use chrono::{DateTime, Days, NaiveTime, TimeDelta, Utc};
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

/// As the owner, creates the heartbeat partition of the day `days` days before `now`.
async fn old_partition(pool: &PgPool, now: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    let old = now - TimeDelta::days(days);
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
    let written = checks::record_beat(&mut conn, id, &beat).await.unwrap();
    assert_eq!(written, checks::Recorded::Written);
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

/// `time` on the UTC day two days before the database's. The ticks after it stay behind the
/// database clock, which bounds the partitions the daily jobs may create and drop (DB-004,
/// DB-005), so the hour of the run cannot change what they do.
async fn two_days_ago_at(pool: &PgPool, time: &str) -> DateTime<Utc> {
    let day = database_now(pool).await.date_naive() - Days::new(2);
    day.and_time(time.parse::<NaiveTime>().unwrap()).and_utc()
}

/// A daily run that fails at `time` is not retried 59 minutes later and is an hour later.
async fn retried_within_the_hour(pool: &PgPool, time: &str) {
    let now = two_days_ago_at(pool, time).await;
    for sql in [
        "CREATE FUNCTION checks.refuse() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        "CREATE TRIGGER refuse BEFORE DELETE ON checks.uptime_minute \
         FOR EACH STATEMENT EXECUTE FUNCTION checks.refuse()",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
    let mut scheduler = scheduler(pool, &Bus::new(16));
    scheduler.tick(now, &mut rng()).await;
    let old = old_partition(pool, now, 40).await;
    sqlx::query("DROP TRIGGER refuse ON checks.uptime_minute")
        .execute(pool)
        .await
        .unwrap();

    scheduler
        .tick(now + TimeDelta::minutes(59), &mut rng())
        .await;
    assert!(partition_exists(pool, old).await, "at {time}");
    scheduler.tick(now + TimeDelta::hours(1), &mut rng()).await;
    assert!(!partition_exists(pool, old).await, "at {time}");
}

tick_test! {
    async fn db_005_the_first_tick_creates_days_ahead_and_drops_old_days(pool: PgPool) {
        let now = database_now(&pool).await;
        let old = old_partition(&pool, now, 40).await;
        let oldest_dropped = old_partition(&pool, now, 31).await;
        let oldest_kept = old_partition(&pool, now, 30).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;

        assert!(!partition_exists(&pool, old).await);
        assert!(!partition_exists(&pool, oldest_dropped).await);
        assert!(partition_exists(&pool, oldest_kept).await);
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

    async fn db_005_a_failed_daily_run_is_retried_within_the_hour_after_midnight(pool: PgPool) {
        retried_within_the_hour(&pool, "00:00:30").await;
    }

    async fn db_005_a_failed_daily_run_is_retried_within_the_hour_before_midnight(pool: PgPool) {
        retried_within_the_hour(&pool, "23:59:30").await;
    }

    async fn cq_021_the_daily_jobs_run_again_when_the_clock_goes_back(pool: PgPool) {
        let now = database_now(&pool).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));
        scheduler.tick(now, &mut rng()).await;
        let old = old_partition(&pool, now, 40).await;

        scheduler.tick(now + TimeDelta::hours(1), &mut rng()).await;
        assert!(partition_exists(&pool, old).await);
        scheduler.tick(now - TimeDelta::hours(1), &mut rng()).await;
        assert!(!partition_exists(&pool, old).await);
    }

    async fn cq_021_a_clock_ahead_of_the_database_trims_no_recent_aggregates(pool: PgPool) {
        let now = database_now(&pool).await;
        let kept = add(&pool, device(DOCKER_HOST), tcp(23), 2).await;
        sqlx::query("INSERT INTO checks.uptime_minute (check_id, bucket) VALUES ($1, $2)")
            .bind(kept)
            .bind(now - TimeDelta::days(1))
            .execute(&pool)
            .await
            .unwrap();
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now + TimeDelta::days(30), &mut rng()).await;

        let minutes = "SELECT count(*) FROM checks.uptime_minute WHERE check_id = $1";
        assert_eq!(count(&pool, minutes, kept).await, 1);
    }
}
