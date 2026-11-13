//! MET-007 to MET-009, R-099: the VRAM of the card readings, and a chart mean that cannot overflow.

use chrono::TimeDelta;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::database::{PVE_01, day_at, device, write};
use crate::fleet::metrics::{MetricKind, MetricRange, Sample, readings, record_collection, series};

const VRAM: MetricKind = MetricKind::VramPercent;

/// Records GPU samples (kind, GPU index, value) of the first fixture device taken at `at`.
async fn write_gpu(pool: &PgPool, at: DateTime<Utc>, values: &[(MetricKind, u8, f64)]) {
    let samples: Vec<Sample> = values
        .iter()
        .map(|&(kind, index, value)| Sample::gpu(kind.into(), index, value).unwrap())
        .collect();
    record_collection(pool, device(PVE_01), at, &samples)
        .await
        .unwrap();
}

/// Stores GPUs 0 and 1 of the first fixture device with the given `has_vram` flags.
async fn store_gpus(pool: &PgPool, has_vram: [bool; 2]) {
    sqlx::query(
        "INSERT INTO metrics.gpus (device_id, index, name, vendor, has_vram) VALUES \
         ($1, 0, 'RTX 4090', 'nvidia', $2), ($1, 1, 'RX 7900', 'amd', $3)",
    )
    .bind(device(PVE_01))
    .bind(has_vram[0])
    .bind(has_vram[1])
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_008_readings_show_the_gpu_with_the_highest_vram_usage(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    store_gpus(&pool, [true, true]).await;
    let at = now - TimeDelta::seconds(20);
    write_gpu(&pool, at, &[(VRAM, 0, 41.0), (VRAM, 1, 77.5)]).await;

    let found = readings(&pool, &[device(PVE_01)], now).await.unwrap();

    let card = &found[&device(PVE_01)];
    assert_eq!(card.vram_percent, Some(77.5));
    let gpu = card.vram_gpu.as_ref().expect("a GPU with VRAM");
    assert_eq!(
        (gpu.index, gpu.name.as_str(), gpu.has_vram),
        (1, "RX 7900", true)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_007_an_integrated_gpu_gives_no_vram_reading_not_zero(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    store_gpus(&pool, [false, false]).await;
    let at = now - TimeDelta::seconds(20);
    write(
        &pool,
        device(PVE_01),
        at,
        &[(MetricKind::CpuPercent, "", 5.0)],
    )
    .await;
    write_gpu(&pool, at, &[(VRAM, 0, 0.0), (VRAM, 1, 12.0)]).await;

    let found = readings(&pool, &[device(PVE_01)], now).await.unwrap();

    let card = &found[&device(PVE_01)];
    assert_eq!((card.vram_percent, &card.vram_gpu), (None, &None));
    assert_eq!(card.cpu_percent, Some(5.0));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_007_a_gpu_with_vram_but_no_sample_gives_no_vram_reading(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    store_gpus(&pool, [true, true]).await;
    let at = now - TimeDelta::seconds(20);
    write(
        &pool,
        device(PVE_01),
        at,
        &[(MetricKind::CpuPercent, "", 5.0)],
    )
    .await;
    write_gpu(&pool, at, &[(MetricKind::GpuPercent, 0, 90.0)]).await;

    let found = readings(&pool, &[device(PVE_01)], now).await.unwrap();

    let card = &found[&device(PVE_01)];
    assert_eq!((card.vram_percent, &card.vram_gpu), (None, &None));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_8_a_vram_sample_older_than_the_window_gives_no_vram_reading(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    store_gpus(&pool, [true, true]).await;
    let pve = device(PVE_01);
    write_gpu(&pool, now - TimeDelta::seconds(180), &[(VRAM, 0, 60.0)]).await;
    write(
        &pool,
        pve,
        now - TimeDelta::seconds(10),
        &[(MetricKind::CpuPercent, "", 3.0)],
    )
    .await;

    let found = readings(&pool, &[pve], now).await.unwrap();

    let card = &found[&pve];
    assert_eq!((card.vram_percent, &card.vram_gpu), (None, &None));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_007_a_vram_series_of_a_gpu_that_is_gone_is_not_shown(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    store_gpus(&pool, [true, true]).await;
    let pve = device(PVE_01);
    write_gpu(
        &pool,
        now - TimeDelta::seconds(20),
        &[(VRAM, 1, 90.0), (VRAM, 0, 30.0)],
    )
    .await;
    sqlx::query("DELETE FROM metrics.gpus WHERE index = 1")
        .execute(&pool)
        .await
        .unwrap();

    let found = readings(&pool, &[pve], now).await.unwrap();

    let card = &found[&pve];
    assert_eq!(card.vram_percent, Some(30.0));
    assert_eq!(card.vram_gpu.as_ref().map(|gpu| gpu.index), Some(0));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_003_a_huge_average_in_a_bin_does_not_fail_the_chart(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    let pve = device(PVE_01);
    let cpu = MetricKind::CpuPercent;
    write(
        &pool,
        pve,
        day_at(&pool, 1, "12:31:10").await,
        &[(cpu, "", 10.0)],
    )
    .await;
    sqlx::query(
        "INSERT INTO metrics.minute (series_id, bucket, avg, min, max, count) \
         SELECT id, $2, 1e308, 1e308, 1e308, 3 FROM metrics.series WHERE device_id = $1",
    )
    .bind(pve)
    .bind(day_at(&pool, 1, "12:32:00").await)
    .execute(&pool)
    .await
    .unwrap();

    let found = series(&pool, pve, MetricRange::Day, now).await.unwrap();

    let points = &found[0].points;
    assert_eq!(points.len(), 1);
    // The ten-minute bin holds 1 sample of 10 and 3 of 1e308: mean 0.75e308.
    assert!(points[0].value.is_finite());
    assert!((points[0].value / 1e308 - 0.75).abs() < 1e-9);
}
