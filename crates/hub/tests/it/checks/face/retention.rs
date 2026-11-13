//! The daily retention run, `checks::drop_expired`: the orphan sweep, aggregate trimming and
//! heartbeat partitions (DEV-015, DB-004, DB-005, DB-009, HUB-005).

use anima_hub::{
    fleet::checks::{self, Error},
    status::Status,
};
use chrono::{DateTime, Days, Months, TimeDelta, Utc};
use sqlx::PgPool;

use super::{PVE_01, beat, count, create, database_now, icmp, new_check, record};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const HEARTBEATS: &str = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";

async fn buckets(pool: &PgPool, table: &str) -> Vec<DateTime<Utc>> {
    sqlx::query_scalar(&format!(
        "SELECT bucket FROM checks.{table} ORDER BY bucket"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

const KEEP_30: Days = Days::new(30);

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

async fn partition_exists(pool: &PgPool, day: DateTime<Utc>) -> bool {
    let name = format!("checks.heartbeats_{}", day.format("%Y%m%d"));
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

devices_test! {
    async fn r_004_the_sweep_deletes_only_the_beats_of_deleted_checks(pool: PgPool) {
        let kept = create(&pool, new_check(PVE_01, "Kept", icmp())).await;
        let gone = create(&pool, new_check(PVE_01, "Gone", icmp())).await;
        let at = database_now(&pool).await;
        for second in 0..3 {
            let each = beat(at + TimeDelta::seconds(second), Status::Up, Some(1));
            record(&pool, kept, &each).await;
            record(&pool, gone, &each).await;
        }
        let mut conn = pool.acquire().await.unwrap();
        checks::delete_check(&mut conn, gone).await.unwrap();
        checks::drop_expired(&pool, at, KEEP_30).await.unwrap();
        assert_eq!((count(&pool, HEARTBEATS, kept).await, count(&pool, HEARTBEATS, gone).await), (3, 0));
        checks::drop_expired(&pool, at, KEEP_30).await.unwrap();
        assert_eq!(count(&pool, HEARTBEATS, kept).await, 3);
    }

    async fn db_009_aggregates_are_trimmed_at_7_days_90_days_and_2_years(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Trimmed", icmp())).await;
        let now = database_now(&pool).await;
        let minute = now - TimeDelta::days(7);
        let hour = now - TimeDelta::days(90);
        let day = now.checked_sub_months(Months::new(24)).unwrap();
        let rows = [
            ("uptime_minute", minute, minute - TimeDelta::minutes(1)),
            ("uptime_hour", hour, hour - TimeDelta::hours(1)),
            ("uptime_day", day, day - TimeDelta::days(1)),
        ];
        for (table, kept, old) in rows {
            for bucket in [old, kept] {
                sqlx::query(&format!("INSERT INTO checks.{table} (check_id, bucket) VALUES ($1, $2)"))
                    .bind(id)
                    .bind(bucket)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }
        checks::drop_expired(&pool, now, KEEP_30).await.unwrap();
        for (table, kept, _) in rows {
            assert_eq!(buckets(&pool, table).await, [kept], "{table}");
        }
    }

    async fn db_004_partitions_are_created_for_today_and_a_week_ahead(pool: PgPool) {
        let today = database_now(&pool).await;
        checks::drop_expired(&pool, today, KEEP_30).await.unwrap();
        for days in 0..=7 {
            assert!(partition_exists(&pool, today + TimeDelta::days(days)).await, "day {days}");
        }
        assert!(!partition_exists(&pool, today + TimeDelta::days(8)).await);
    }

    async fn db_004_a_clock_far_ahead_of_the_database_fails_the_run(pool: PgPool) {
        let today = database_now(&pool).await;
        let error = checks::drop_expired(&pool, today + TimeDelta::days(30), KEEP_30)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Database(_)), "{error:?}");
    }

    async fn hub_005_heartbeat_days_older_than_the_period_are_dropped_and_newer_kept(pool: PgPool) {
        let now = database_now(&pool).await;
        let mut days = Vec::new();
        for ago in [40, 12, 9, 3] {
            days.push((ago, old_partition(&pool, now, ago).await));
        }

        checks::drop_expired(&pool, now, KEEP_30).await.unwrap();
        let after_30 = [partition_exists(&pool, days[0].1).await, partition_exists(&pool, days[1].1).await];
        checks::drop_expired(&pool, now, Days::new(10)).await.unwrap();
        let after_10 = [partition_exists(&pool, days[1].1).await, partition_exists(&pool, days[2].1).await];
        checks::drop_expired(&pool, now, Days::new(1)).await.unwrap();
        let after_1 = [partition_exists(&pool, days[2].1).await, partition_exists(&pool, days[3].1).await];

        assert_eq!(after_30, [false, true], "30 days drops 40 days ago and keeps 12");
        assert_eq!(after_10, [false, true], "10 days drops 12 days ago and keeps 9");
        assert_eq!(after_1, [false, false], "1 day drops 9 and 3 days ago");
        assert!(partition_exists(&pool, now).await, "today is never dropped");
    }
}
