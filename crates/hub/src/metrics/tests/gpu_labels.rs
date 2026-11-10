//! R-099: the chart label of a GPU series; MET-008: which GPU the card readings show.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::database::{PVE_01, day_at, device, write};
use crate::metrics::{
    Gpu, GpuVendor, MetricKind, MetricRange, Sample, gpu_label::chart_label, latest,
    record_collection, series,
};

fn gpu(index: i32, has_vram: bool) -> Gpu {
    Gpu {
        index,
        name: format!("GPU-{index}"),
        vendor: GpuVendor::Nvidia,
        has_vram,
    }
}

/// The VRAM reading of a device whose latest VRAM samples are `(GPU, percent)`, in that order.
fn vram_of(samples: &[(Gpu, f64)]) -> (Option<f64>, Option<i32>) {
    let at: DateTime<Utc> = DateTime::UNIX_EPOCH;
    let latest = samples
        .iter()
        .map(|(gpu, value)| latest::Latest {
            device: device(PVE_01),
            kind: MetricKind::VramPercent,
            at,
            value: *value,
            gpu: Some(gpu.clone()),
        })
        .collect();
    let card = &latest::readings(latest)[&device(PVE_01)];
    (
        card.vram_percent,
        card.vram_gpu.as_ref().map(|gpu| gpu.index),
    )
}

#[test]
fn r_099_a_gpu_series_is_labelled_with_its_index_and_name_else_its_index_alone() {
    assert_eq!(
        chart_label("1", Some(1), Some("RTX 4090")),
        "GPU 1 RTX 4090"
    );
    assert_eq!(chart_label("1", Some(1), None), "GPU 1");
    assert_eq!(chart_label("/data", None, None), "/data");
}

#[test]
fn met_008_the_highest_vram_wins_whatever_the_row_order() {
    let (a, b) = (gpu(0, true), gpu(1, true));
    assert_eq!(
        vram_of(&[(a.clone(), 80.0), (b.clone(), 20.0)]),
        (Some(80.0), Some(0))
    );
    assert_eq!(vram_of(&[(b, 20.0), (a, 80.0)]), (Some(80.0), Some(0)));
}

#[test]
fn met_008_a_tie_keeps_the_lower_gpu_index_whatever_the_row_order() {
    let (a, b) = (gpu(0, true), gpu(1, true));
    assert_eq!(
        vram_of(&[(a.clone(), 50.0), (b.clone(), 50.0)]),
        (Some(50.0), Some(0))
    );
    assert_eq!(vram_of(&[(b, 50.0), (a, 50.0)]), (Some(50.0), Some(0)));
}

#[test]
fn met_007_a_gpu_without_vram_never_beats_one_with_vram() {
    let integrated = gpu(0, false);
    let discrete = gpu(1, true);
    assert_eq!(
        vram_of(&[(integrated, 95.0), (discrete, 30.0)]),
        (Some(30.0), Some(1))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn r_099_a_chart_series_of_a_gpu_is_labelled_from_the_stored_gpu(pool: PgPool) {
    let now = day_at(&pool, 1, "12:34:30").await;
    let pve = device(PVE_01);
    sqlx::query(
        "INSERT INTO metrics.gpus (device_id, index, name, vendor, has_vram) \
         VALUES ($1, 0, 'RTX 4090', 'nvidia', true)",
    )
    .bind(pve)
    .execute(&pool)
    .await
    .unwrap();
    let at = now - TimeDelta::seconds(20);
    let samples: Vec<Sample> = [0, 1]
        .map(|index| Sample::gpu(MetricKind::GpuPercent.into(), index, 40.0).unwrap())
        .into();
    record_collection(&pool, pve, at, &samples).await.unwrap();
    write(&pool, pve, at, &[(MetricKind::DiskPercent, "/", 30.0)]).await;

    let found = series(&pool, pve, MetricRange::Hour, now).await.unwrap();

    let shown: Vec<_> = found
        .iter()
        .map(|each| (each.label.as_str(), each.gpu_index))
        .collect();
    // `1` has no stored GPU any more, so it is "GPU 1".
    assert_eq!(
        shown,
        [("/", None), ("GPU 0 RTX 4090", Some(0)), ("GPU 1", Some(1))]
    );
}
