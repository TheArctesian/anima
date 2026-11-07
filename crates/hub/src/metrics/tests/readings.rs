//! D-8 and MET-006: the card readings and the stored GPUs.

use chrono::TimeDelta;
use sqlx::PgPool;

use super::database::{DOCKER_HOST, PVE_01, day_at, device, write};
use crate::metrics::{Gpu, GpuVendor, MetricKind, gpus, readings};

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
