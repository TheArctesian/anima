//! DB-008: each sample lands in its minute, hour and day row.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    assert_close,
    database::{DOCKER_HOST, PVE_01, day_at, device, rows, write},
};
use crate::{ids::DeviceId, metrics::MetricKind};

const CPU: MetricKind = MetricKind::CpuPercent;

/// The (avg, min, max, count) of the row of `table` for the CPU series of `device`.
async fn bucket(
    pool: &PgPool,
    table: &str,
    device: DeviceId,
    start: DateTime<Utc>,
) -> Option<(f64, f64, f64, i32)> {
    sqlx::query_as(&format!(
        "SELECT a.avg, a.min, a.max, a.count FROM metrics.{table} a \
         JOIN metrics.series s ON s.id = a.series_id \
         WHERE s.device_id = $1 AND s.kind = 'cpu_percent' AND a.bucket = $2"
    ))
    .bind(device)
    .bind(start)
    .fetch_optional(pool)
    .await
    .unwrap()
}

fn assert_bucket(found: Option<(f64, f64, f64, i32)>, expected: (f64, f64, f64, i32)) {
    let (avg, min, max, count) = found.expect("the bucket exists");
    assert_close(avg, expected.0);
    assert_close(min, expected.1);
    assert_close(max, expected.2);
    assert_eq!(count, expected.3);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_008_two_samples_in_a_minute_share_one_row_with_their_mean_min_max_and_count(
    pool: PgPool,
) {
    let pve = device(PVE_01);
    write(
        &pool,
        pve,
        day_at(&pool, 1, "10:05:10").await,
        &[(CPU, "", 10.0)],
    )
    .await;
    write(
        &pool,
        pve,
        day_at(&pool, 1, "10:05:40").await,
        &[(CPU, "", 30.0)],
    )
    .await;

    let minute = bucket(&pool, "minute", pve, day_at(&pool, 1, "10:05:00").await).await;
    assert_bucket(minute, (20.0, 10.0, 30.0, 2));
    assert_eq!(rows(&pool, "minute").await, 1);
    assert_eq!(rows(&pool, "series").await, 1);
    assert_eq!(rows(&pool, "samples").await, 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_008_minutes_hours_and_days_each_keep_their_own_row(pool: PgPool) {
    let pve = device(PVE_01);
    for (days_ago, time, value) in [
        (1, "10:05:10", 10.0),
        (1, "10:05:40", 30.0),
        (1, "10:25:00", 80.0),
        (1, "11:00:00", 5.0),
        (0, "00:00:05", 50.0),
    ] {
        write(
            &pool,
            pve,
            day_at(&pool, days_ago, time).await,
            &[(CPU, "", value)],
        )
        .await;
    }

    let at = async |days_ago, time| day_at(&pool, days_ago, time).await;
    assert_bucket(
        bucket(&pool, "minute", pve, at(1, "10:25:00").await).await,
        (80.0, 80.0, 80.0, 1),
    );
    assert_bucket(
        bucket(&pool, "hour", pve, at(1, "10:00:00").await).await,
        (40.0, 10.0, 80.0, 3),
    );
    assert_bucket(
        bucket(&pool, "hour", pve, at(1, "11:00:00").await).await,
        (5.0, 5.0, 5.0, 1),
    );
    assert_bucket(
        bucket(&pool, "day", pve, at(1, "00:00:00").await).await,
        (31.25, 5.0, 80.0, 4),
    );
    assert_bucket(
        bucket(&pool, "day", pve, at(0, "00:00:00").await).await,
        (50.0, 50.0, 50.0, 1),
    );
    assert_eq!(rows(&pool, "minute").await, 4);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_008_writing_the_same_instant_again_adds_nothing(pool: PgPool) {
    let pve = device(PVE_01);
    let at = day_at(&pool, 1, "10:05:10").await;
    write(&pool, pve, at, &[(CPU, "", 10.0)]).await;
    write(&pool, pve, at, &[(CPU, "", 90.0)]).await;

    let minute = bucket(&pool, "minute", pve, day_at(&pool, 1, "10:05:00").await).await;
    assert_bucket(minute, (10.0, 10.0, 10.0, 1));
    assert_eq!(rows(&pool, "samples").await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_008_one_collection_writes_a_series_per_kind_and_label_and_device(pool: PgPool) {
    let at = day_at(&pool, 1, "10:05:10").await;
    let values = [
        (CPU, "", 10.0),
        (MetricKind::DiskPercent, "/", 40.0),
        (MetricKind::DiskPercent, "/data", 60.0),
        (MetricKind::NetworkInBps, "eth0", 1234.5),
    ];
    write(&pool, device(PVE_01), at, &values).await;
    write(&pool, device(DOCKER_HOST), at, &values[..1]).await;
    write(
        &pool,
        device(PVE_01),
        day_at(&pool, 1, "10:05:40").await,
        &values,
    )
    .await;

    assert_eq!(rows(&pool, "series").await, 5);
    assert_eq!(rows(&pool, "samples").await, 9);
    assert_eq!(rows(&pool, "hour").await, 5);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_004_a_sample_beyond_the_partition_window_is_refused_and_nothing_is_written(
    pool: PgPool,
) {
    let pve = device(PVE_01);
    let far = day_at(&pool, 0, "10:00:00").await + chrono::TimeDelta::days(9);
    let sample = crate::metrics::Sample::new(CPU, "", 1.0).unwrap();

    let refused = crate::metrics::record_collection(&pool, pve, far, &[sample]).await;

    assert!(matches!(refused, Err(crate::metrics::Error::Database(_))));
    assert_eq!(rows(&pool, "series").await, 0);
    assert_eq!(rows(&pool, "samples").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_008_a_collection_for_a_missing_device_says_so(pool: PgPool) {
    let at = day_at(&pool, 1, "10:05:10").await;
    let sample = crate::metrics::Sample::new(CPU, "", 1.0).unwrap();

    let missing = crate::metrics::record_collection(&pool, device(PVE_01), at, &[sample]).await;

    assert!(matches!(missing, Err(crate::metrics::Error::NoDevice)));
    assert_eq!(rows(&pool, "samples").await, 0);
}
