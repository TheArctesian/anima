//! DB-004, DB-005, DB-009, DEV-015 and HUB-005: partitions kept ahead, partitions and rows
//! dropped when they expire, and the samples of deleted series.

use chrono::{DateTime, Days, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::database::{PVE_01, database_now, day_at, device, rows, write};
use crate::{
    ids::DeviceId,
    metrics::{MetricKind, Retention, create_partitions_ahead, drop_expired_partitions},
};

/// The names of the day partitions of `metrics.samples`, oldest first.
async fn partitions(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid \
         WHERE i.inhparent = 'metrics.samples'::regclass ORDER BY c.relname",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn name_of(at: DateTime<Utc>) -> String {
    format!("samples_{}", at.format("%Y%m%d"))
}

/// A series of `device` and its aggregate rows `age` before `now`, one row per table.
async fn aged_aggregates(pool: &PgPool, device: DeviceId, kind: &str, ages: [TimeDelta; 3]) {
    let series = Uuid::now_v7();
    sqlx::query("INSERT INTO metrics.series (id, device_id, kind) VALUES ($1, $2, $3)")
        .bind(series)
        .bind(device)
        .bind(kind)
        .execute(pool)
        .await
        .unwrap();
    let now = database_now(pool).await;
    for (table, age) in ["minute", "hour", "day"].into_iter().zip(ages) {
        sqlx::query(&format!(
            "INSERT INTO metrics.{table} (series_id, bucket, avg, min, max) VALUES ($1, $2, 1, 1, 1)"
        ))
        .bind(series)
        .bind(now - age)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_004_partitions_are_created_for_today_and_seven_days_ahead_and_again_safely(
    pool: PgPool,
) {
    let now = database_now(&pool).await;

    create_partitions_ahead(&pool, now).await.unwrap();
    create_partitions_ahead(&pool, now).await.unwrap();

    let expected: Vec<String> = (0..=7).map(|days| name_of(now + Days::new(days))).collect();
    assert_eq!(partitions(&pool).await, expected);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_004_a_sample_in_a_day_without_a_partition_is_refused(pool: PgPool) {
    sqlx::query("INSERT INTO metrics.series (id, device_id, kind) VALUES ($1, $2, 'cpu_percent')")
        .bind(Uuid::now_v7())
        .bind(device(PVE_01))
        .execute(&pool)
        .await
        .unwrap();

    let refused = sqlx::query(
        "INSERT INTO metrics.samples (series_id, at, value) \
         SELECT id, now(), 1 FROM metrics.series",
    )
    .execute(&pool)
    .await
    .unwrap_err();

    let code = refused
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    assert_eq!(code.as_deref(), Some("23514"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_005_a_partition_past_30_days_is_dropped_and_a_recent_one_stays(pool: PgPool) {
    let pve = device(PVE_01);
    write(
        &pool,
        pve,
        day_at(&pool, 1, "10:00:00").await,
        &[(MetricKind::CpuPercent, "", 1.0)],
    )
    .await;
    sqlx::query(
        "CREATE TABLE metrics.samples_20200101 PARTITION OF metrics.samples \
         FOR VALUES FROM ('2020-01-01 00:00:00+00') TO ('2020-01-02 00:00:00+00')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO metrics.samples (series_id, at, value) \
         SELECT id, '2020-01-01 10:00:00+00', 1 FROM metrics.series",
    )
    .execute(&pool)
    .await
    .unwrap();
    let now = database_now(&pool).await;

    let trimmed = drop_expired_partitions(&pool, now, Retention::default())
        .await
        .unwrap();

    assert_eq!(trimmed.partitions, ["samples_20200101"]);
    assert_eq!(partitions(&pool).await, [name_of(now - TimeDelta::days(1))]);
    assert_eq!(rows(&pool, "samples").await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_009_aggregates_go_after_7_days_90_days_and_2_years(pool: PgPool) {
    let pve = device(PVE_01);
    let days = TimeDelta::days;
    aged_aggregates(&pool, pve, "cpu_percent", [days(8), days(91), days(731)]).await;
    aged_aggregates(&pool, pve, "load_1", [days(6), days(89), days(729)]).await;

    let trimmed = drop_expired_partitions(&pool, database_now(&pool).await, Retention::default())
        .await
        .unwrap();

    assert_eq!((trimmed.minute, trimmed.hour, trimmed.day), (1, 1, 1));
    for table in ["minute", "hour", "day"] {
        assert_eq!(rows(&pool, table).await, 1, "{table}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn hub_005_the_periods_come_from_the_retention_passed_in(pool: PgPool) {
    let pve = device(PVE_01);
    let days = TimeDelta::days;
    aged_aggregates(&pool, pve, "cpu_percent", [days(2), days(10), days(100)]).await;
    let now = database_now(&pool).await;

    let kept = drop_expired_partitions(&pool, now, Retention::default())
        .await
        .unwrap();
    let shorter = Retention {
        minute: days(1),
        hour: days(5),
        day: days(50),
        ..Retention::default()
    };
    let trimmed = drop_expired_partitions(&pool, now, shorter).await.unwrap();

    assert_eq!((kept.minute, kept.hour, kept.day), (0, 0, 0));
    assert_eq!((trimmed.minute, trimmed.hour, trimmed.day), (1, 1, 1));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_005_a_retention_shorter_than_30_days_cannot_drop_a_recent_partition(pool: PgPool) {
    let pve = device(PVE_01);
    write(
        &pool,
        pve,
        day_at(&pool, 3, "10:00:00").await,
        &[(MetricKind::CpuPercent, "", 1.0)],
    )
    .await;
    let early = Retention {
        samples: TimeDelta::days(1),
        ..Retention::default()
    };

    let refused = drop_expired_partitions(&pool, database_now(&pool).await, early).await;

    assert!(refused.is_err());
    assert_eq!(partitions(&pool).await.len(), 1);
    assert_eq!(rows(&pool, "samples").await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn dev_015_deleting_a_device_removes_its_series_and_aggregates_and_retention_its_samples(
    pool: PgPool,
) {
    let (pve, docker) = (device(PVE_01), device(super::database::DOCKER_HOST));
    let at = day_at(&pool, 1, "10:00:00").await;
    write(
        &pool,
        pve,
        at,
        &[
            (MetricKind::CpuPercent, "", 1.0),
            (MetricKind::Load1, "", 2.0),
        ],
    )
    .await;
    write(&pool, docker, at, &[(MetricKind::CpuPercent, "", 3.0)]).await;
    sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(pve)
        .execute(&pool)
        .await
        .unwrap();

    let after_delete = (
        rows(&pool, "series").await,
        rows(&pool, "minute").await,
        rows(&pool, "samples").await,
    );
    let trimmed = drop_expired_partitions(&pool, database_now(&pool).await, Retention::default())
        .await
        .unwrap();

    assert_eq!(after_delete, (1, 1, 3));
    assert_eq!(trimmed.orphan_samples, 2);
    assert_eq!(rows(&pool, "samples").await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn d5_rotating_labels_leave_only_the_current_series_after_retention(pool: PgPool) {
    let pve = device(PVE_01);
    let disk = MetricKind::DiskPercent;
    for (days_ago, label) in [(10, "/old1"), (10, "/old2"), (9, "/old3"), (1, "/now")] {
        let at = day_at(&pool, days_ago, "10:00:00").await;
        write(&pool, pve, at, &[(disk, label, 1.0)]).await;
    }
    let short = Retention {
        minute: TimeDelta::days(5),
        hour: TimeDelta::days(5),
        day: TimeDelta::days(5),
        ..Retention::default()
    };

    let trimmed = drop_expired_partitions(&pool, database_now(&pool).await, short)
        .await
        .unwrap();
    let left: Vec<String> = sqlx::query_scalar("SELECT label FROM metrics.series")
        .fetch_all(&pool)
        .await
        .unwrap();
    // A label that comes back is a new series again.
    write(
        &pool,
        pve,
        day_at(&pool, 0, "01:00:00").await,
        &[(disk, "/old1", 2.0)],
    )
    .await;

    assert_eq!(trimmed.idle_series, 3);
    assert_eq!(left, ["/now"]);
    assert_eq!(rows(&pool, "series").await, 2);
    assert_eq!(rows(&pool, "samples").await, 2);
}
