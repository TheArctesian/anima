//! The read faces: MET-003 and D-7 (chart ranges), D-8 (card readings), MET-006 (GPUs).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{
    assert_close,
    database::{DOCKER_HOST, PVE_01, day_at, device, write},
};
use crate::metrics::{
    Gpu, GpuVendor, MetricKind, MetricPoint, MetricRange, MetricSeries, gpus, readings, series,
};

const CPU: MetricKind = MetricKind::CpuPercent;

/// 34 minutes and 30 seconds past 12:00 on the UTC day before the database's.
async fn now(pool: &PgPool) -> DateTime<Utc> {
    day_at(pool, 1, "12:34:30").await
}

/// CPU samples around `now`: a lone one an hour back, a pair in the minute of 12:34 and a
/// third in the next minute (12:31 is before them).
async fn write_history(pool: &PgPool, now: DateTime<Utc>) {
    let pve = device(PVE_01);
    for (before, value) in [
        (TimeDelta::seconds(60 * 60), 99.0),
        (TimeDelta::seconds(59 * 60 + 30), 5.0),
        (TimeDelta::seconds(20), 10.0),
        (TimeDelta::seconds(10), 20.0),
        (TimeDelta::seconds(3 * 60 + 20), 60.0),
    ] {
        write(pool, pve, now - before, &[(CPU, "", value)]).await;
    }
}

fn point(at: DateTime<Utc>, value: f64, min: f64, max: f64) -> MetricPoint {
    MetricPoint {
        at,
        value,
        min: Some(min),
        max: Some(max),
    }
}

fn assert_points(series: &MetricSeries, expected: &[MetricPoint]) {
    assert_eq!(series.points.len(), expected.len(), "{:?}", series.points);
    for (found, wanted) in series.points.iter().zip(expected) {
        assert_eq!(found.at, wanted.at);
        assert_close(found.value, wanted.value);
        assert_eq!((found.min, found.max), (wanted.min, wanted.max));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_003_hour_reads_minute_rows_of_the_last_60_minutes(pool: PgPool) {
    let now = now(&pool).await;
    write_history(&pool, now).await;

    let found = series(&pool, device(PVE_01), MetricRange::Hour, now)
        .await
        .unwrap();

    // 11:34:30 is before the window, which starts at the minute 11:35.
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].kind, found[0].label.as_str()), (CPU, ""));
    assert_eq!(found[0].gpu_index, None);
    let at = async |time| day_at(&pool, 1, time).await;
    assert_points(
        &found[0],
        &[
            point(at("11:35:00").await, 5.0, 5.0, 5.0),
            point(at("12:31:00").await, 60.0, 60.0, 60.0),
            point(at("12:34:00").await, 15.0, 10.0, 20.0),
        ],
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_003_day_bins_minutes_to_ten_minutes_weighting_the_mean_by_count(pool: PgPool) {
    let now = now(&pool).await;
    write_history(&pool, now).await;
    let pve = device(PVE_01);
    // The oldest bin of the day range starts 23 h 50 min before 12:30. One sample sits inside
    // it, one a minute before it.
    let oldest = day_at(&pool, 2, "12:40:00").await;
    write(
        &pool,
        pve,
        oldest + TimeDelta::seconds(30),
        &[(CPU, "", 7.0)],
    )
    .await;
    write(
        &pool,
        pve,
        oldest - TimeDelta::seconds(30),
        &[(CPU, "", 88.0)],
    )
    .await;

    let found = series(&pool, pve, MetricRange::Day, now).await.unwrap();

    // The minute rows of 12:31 (60) and 12:34 (10 and 20, mean 15, count 2) share the 12:30
    // bin: (60 + 15 * 2) / 3 = 30, not the mean of the means.
    let at = async |days_ago, time| day_at(&pool, days_ago, time).await;
    assert_points(
        &found[0],
        &[
            point(oldest, 7.0, 7.0, 7.0),
            point(at(1, "11:30:00").await, 52.0, 5.0, 99.0),
            point(at(1, "12:30:00").await, 30.0, 10.0, 60.0),
        ],
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_003_week_reads_hour_rows_of_the_last_168_hours(pool: PgPool) {
    let now = now(&pool).await;
    write_history(&pool, now).await;
    let pve = device(PVE_01);
    // The oldest hour starts 167 h before 12:00; a sample of the hour before it is out.
    let oldest = day_at(&pool, 1, "12:00:00").await - TimeDelta::hours(167);
    write(
        &pool,
        pve,
        oldest + TimeDelta::minutes(10),
        &[(CPU, "", 7.0)],
    )
    .await;
    write(
        &pool,
        pve,
        oldest - TimeDelta::minutes(10),
        &[(CPU, "", 88.0)],
    )
    .await;

    let found = series(&pool, pve, MetricRange::Week, now).await.unwrap();

    let at = async |time| day_at(&pool, 1, time).await;
    assert_points(
        &found[0],
        &[
            point(oldest, 7.0, 7.0, 7.0),
            point(at("11:00:00").await, 52.0, 5.0, 99.0),
            point(at("12:00:00").await, 30.0, 10.0, 60.0),
        ],
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_003_series_come_one_per_kind_and_label_and_only_for_the_device(pool: PgPool) {
    let now = now(&pool).await;
    let at = now - TimeDelta::seconds(30);
    write(
        &pool,
        device(PVE_01),
        at,
        &[
            (MetricKind::DiskPercent, "/data", 60.0),
            (MetricKind::DiskPercent, "/", 40.0),
            (CPU, "", 10.0),
        ],
    )
    .await;
    write(&pool, device(DOCKER_HOST), at, &[(CPU, "", 1.0)]).await;

    let found = series(&pool, device(PVE_01), MetricRange::Hour, now)
        .await
        .unwrap();

    let names: Vec<(MetricKind, &str)> = found
        .iter()
        .map(|each| (each.kind, each.label.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            (CPU, ""),
            (MetricKind::DiskPercent, "/"),
            (MetricKind::DiskPercent, "/data")
        ]
    );
    let none = series(
        &pool,
        device(PVE_01),
        MetricRange::Hour,
        now - TimeDelta::hours(5),
    )
    .await
    .unwrap();
    assert_eq!(none.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_004_readings_hold_the_latest_values_and_the_fullest_disk(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    let pve = device(PVE_01);
    let seconds = TimeDelta::seconds;
    write(
        &pool,
        pve,
        now - seconds(100),
        &[(MetricKind::CpuPercent, "", 10.0)],
    )
    .await;
    write(
        &pool,
        pve,
        now - seconds(30),
        &[
            (MetricKind::CpuPercent, "", 40.0),
            (MetricKind::DiskPercent, "/", 30.0),
            (MetricKind::DiskPercent, "/data", 80.0),
            (MetricKind::Load1, "", 2.0),
        ],
    )
    .await;
    write(
        &pool,
        pve,
        now - seconds(10),
        &[(MetricKind::DiskPercent, "/", 31.0)],
    )
    .await;
    // Three minutes old: not a reading.
    write(
        &pool,
        pve,
        now - seconds(180),
        &[(MetricKind::MemoryPercent, "", 55.0)],
    )
    .await;

    let found = readings(&pool, &[pve], now).await.unwrap();

    let card = &found[&pve];
    assert_eq!(card.at, now - seconds(10));
    assert_eq!(card.cpu_percent, Some(40.0));
    assert_eq!(card.memory_percent, None);
    assert_eq!(card.disk_percent, Some(80.0));
    assert_eq!((card.vram_percent, &card.vram_gpu), (None, &None));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_004_a_device_with_only_old_samples_has_no_readings(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    let (pve, docker) = (device(PVE_01), device(DOCKER_HOST));
    write(
        &pool,
        pve,
        now - TimeDelta::seconds(180),
        &[(MetricKind::CpuPercent, "", 1.0)],
    )
    .await;
    write(
        &pool,
        docker,
        now - TimeDelta::seconds(119),
        &[(MetricKind::CpuPercent, "", 2.0)],
    )
    .await;
    // After `now` is not yet a reading either.
    write(
        &pool,
        pve,
        now + TimeDelta::seconds(5),
        &[(MetricKind::MemoryPercent, "", 3.0)],
    )
    .await;

    let found = readings(&pool, &[pve, docker], now).await.unwrap();

    assert_eq!(found.keys().copied().collect::<Vec<_>>(), [docker]);
    assert_eq!(found[&docker].cpu_percent, Some(2.0));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_006_gpus_are_the_stored_rows_by_index_per_device(pool: PgPool) {
    sqlx::query(
        "INSERT INTO metrics.gpus (device_id, index, name, vendor, has_vram) VALUES \
         ($1, 1, 'Intel UHD', 'intel', false), ($1, 0, 'RTX 4090', 'nvidia', true), \
         ($2, 0, 'Apple M2', 'apple', false)",
    )
    .bind(device(PVE_01))
    .bind(device(DOCKER_HOST))
    .execute(&pool)
    .await
    .unwrap();
    let none = device("0190a000-0000-7000-8002-000000000003");

    let found = gpus(&pool, &[device(PVE_01), device(DOCKER_HOST), none])
        .await
        .unwrap();

    let gpu = |index, name: &str, vendor, has_vram| Gpu {
        index,
        name: name.to_owned(),
        vendor,
        has_vram,
    };
    assert_eq!(
        found[&device(PVE_01)],
        [
            gpu(0, "RTX 4090", GpuVendor::Nvidia, true),
            gpu(1, "Intel UHD", GpuVendor::Intel, false)
        ]
    );
    assert_eq!(found[&device(DOCKER_HOST)].len(), 1);
    assert!(!found.contains_key(&none));
}
