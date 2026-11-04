//! The daily jobs' store functions: the orphan sweep, aggregate trimming and heartbeat
//! partitions (DEV-015, DB-004, DB-005, DB-009).

use anima_hub::{
    checks::{self, Error, Trimmed},
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
        assert_eq!(checks::delete_orphan_heartbeats(&pool).await.unwrap(), 3);
        assert_eq!((count(&pool, HEARTBEATS, kept).await, count(&pool, HEARTBEATS, gone).await), (3, 0));
        assert_eq!(checks::delete_orphan_heartbeats(&pool).await.unwrap(), 0);
    }

    async fn db_009_aggregates_are_trimmed_at_7_days_90_days_and_2_years(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Trimmed", icmp())).await;
        let now: DateTime<Utc> = "2026-11-04T09:00:00Z".parse().unwrap();
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
        let trimmed = checks::trim_aggregates(&pool, now).await.unwrap();
        assert_eq!(trimmed, Trimmed { minute: 1, hour: 1, day: 1 });
        for (table, kept, _) in rows {
            assert_eq!(buckets(&pool, table).await, [kept], "{table}");
        }
    }

    async fn db_004_partitions_are_created_for_today_and_the_days_ahead(pool: PgPool) {
        let today = database_now(&pool).await;
        checks::create_days_ahead(&pool, today, 3).await.unwrap();
        for days in 0..=3 {
            assert!(partition_exists(&pool, today + TimeDelta::days(days)).await, "day {days}");
        }
        assert!(!partition_exists(&pool, today + TimeDelta::days(4)).await);
        let error = checks::create_days_ahead(&pool, today, 30).await.unwrap_err();
        assert!(matches!(error, Error::Database(_)), "{error:?}");
    }

    async fn db_005_old_heartbeat_days_are_dropped_and_recent_ones_refused(pool: PgPool) {
        let now = database_now(&pool).await;
        let old = (now - TimeDelta::days(40)).date_naive();
        let next = old.succ_opt().unwrap();
        sqlx::query(&format!(
            "CREATE TABLE checks.heartbeats_{} PARTITION OF checks.heartbeats \
             FOR VALUES FROM ('{old}T00:00:00Z') TO ('{next}T00:00:00Z')",
            old.format("%Y%m%d")
        ))
        .execute(&pool)
        .await
        .unwrap();
        checks::create_days_ahead(&pool, now, 0).await.unwrap();
        let today = now.date_naive();
        let refused = checks::drop_heartbeat_days_before(&pool, today - Days::new(29), now).await;
        assert!(matches!(refused, Err(Error::Database(_))), "{refused:?}");
        let dropped = checks::drop_heartbeat_days_before(&pool, today - Days::new(30), now).await.unwrap();
        assert_eq!(dropped, [format!("heartbeats_{}", old.format("%Y%m%d"))]);
        assert!(partition_exists(&pool, now).await);
    }
}
