//! R-099, R-092 D-6: a GPU series stores its GPU index and the index as its label, every other
//! kind stores none, and a `gpu_memory_bytes` series is stored and read back.

use agent_core::points::{Kind, Sample};
use sqlx::PgPool;

use super::database::{PVE_01, database_now, device};
use crate::metrics::{MetricKind, MetricRange, record_collection, series};

/// Inserts one series row of pve-01 as a writer could, and whether the database took it.
async fn stored(pool: &PgPool, kind: &str, label: &str, gpu_index: Option<i32>) -> bool {
    sqlx::query(
        "INSERT INTO metrics.series (id, device_id, kind, label, gpu_index, created_at) \
         VALUES (gen_random_uuid(), $1, $2, $3, $4, now())",
    )
    .bind(device(PVE_01))
    .bind(kind)
    .bind(label)
    .bind(gpu_index)
    .execute(pool)
    .await
    .is_ok()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn r_099_a_gpu_series_has_its_index_as_label_and_no_other_kind_has_an_index(pool: PgPool) {
    assert!(stored(&pool, "gpu_percent", "0", Some(0)).await);
    assert!(stored(&pool, "gpu_memory_bytes", "12", Some(12)).await);
    assert!(
        !stored(&pool, "vram_bytes", "", Some(0)).await,
        "label not the index"
    );
    assert!(
        !stored(&pool, "vram_bytes", "1", Some(0)).await,
        "another index"
    );
    assert!(
        !stored(&pool, "gpu_power_w", "0", None).await,
        "a GPU kind with no index"
    );
    assert!(
        !stored(&pool, "cpu_percent", "", Some(0)).await,
        "an index on a plain kind"
    );
    assert!(
        !stored(&pool, "disk_percent", "0", Some(0)).await,
        "an index on a disk"
    );
    assert!(
        stored(&pool, "disk_percent", "0", None).await,
        "a disk may be named 0"
    );
    assert!(
        !stored(&pool, "gpu_bogus", "0", Some(0)).await,
        "an unknown kind"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn r_092_d_6_gpu_memory_bytes_is_stored_and_read_back_per_gpu(pool: PgPool) {
    let at = database_now(&pool).await - chrono::TimeDelta::seconds(30);
    let samples = [
        Sample::gpu(Kind::GpuMemoryBytes, 0, 8.0e9).unwrap(),
        Sample::gpu(Kind::GpuMemoryBytes, 1, 2.0e9).unwrap(),
    ];
    record_collection(&pool, device(PVE_01), at, &samples)
        .await
        .unwrap();

    let found = series(&pool, device(PVE_01), MetricRange::Hour, at)
        .await
        .unwrap();
    let read: Vec<(MetricKind, &str, Option<i32>, f64)> = found
        .iter()
        .map(|one| {
            (
                one.kind,
                one.label.as_str(),
                one.gpu_index,
                one.points[0].value,
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (MetricKind::GpuMemoryBytes, "GPU 0", Some(0), 8.0e9),
            (MetricKind::GpuMemoryBytes, "GPU 1", Some(1), 2.0e9),
        ]
    );
}
