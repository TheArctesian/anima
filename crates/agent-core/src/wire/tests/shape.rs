//! The point, value and GPU rules of a batch (K8 Section 4): each refuses its own case only.

use chrono::TimeDelta;

use super::*;
use crate::wire::{BatchGpu, BatchPoint, GpuVendor};

/// `count` points ending at the example's newest, 10 seconds apart, each with one CPU mean.
fn cpu_points(count: i32) -> impl FnOnce(&mut Batch) {
    move |batch: &mut Batch| {
        batch.series.truncate(1);
        let newest = at("2026-11-08T10:00:20Z");
        batch.points = (0..count)
            .rev()
            .map(|n| BatchPoint {
                at: newest - TimeDelta::seconds(10 * i64::from(n)),
                mean: vec![Some(1.0)],
                max: None,
            })
            .collect();
    }
}

#[test]
fn k8_4_a_batch_has_1_to_360_points() {
    assert_malformed("no points", |batch| batch.points.clear());
    assert_accepted("one point", |batch| batch.points.truncate(1));
    assert_accepted("360 points", cpu_points(360));
    assert_malformed("361 points", cpu_points(361));
}

#[test]
fn k8_4_points_are_strictly_increasing_on_multiples_of_10_seconds() {
    assert_malformed("the same time twice", |batch| {
        batch.points[1].at = batch.points[0].at;
    });
    assert_malformed("going back", |batch| batch.points.swap(0, 1));
    assert_malformed("15 seconds", |batch| {
        batch.points[1].at = at("2026-11-08T10:00:15Z");
    });
    assert_malformed("a fraction", |batch| {
        batch.points[1].at = at("2026-11-08T10:00:20.5Z");
    });
    assert_accepted("a gap", |batch| {
        batch.points[0].at = at("2026-11-08T09:59:40Z");
    });
}

#[test]
fn k8_4_mean_and_max_have_one_entry_per_series() {
    assert_malformed("a short mean", |batch| {
        batch.points[0].mean.pop();
    });
    assert_malformed("a long mean", |batch| batch.points[1].mean.push(None));
    assert_malformed("a short max", |batch| {
        batch.points[0].max.as_mut().unwrap().pop();
    });
    assert_malformed("an empty max", |batch| {
        batch.points[1].max = Some(Vec::new());
    });
    assert_accepted("an all-null max", |batch| {
        batch.points[1].max = Some(vec![None; 3]);
    });
}

#[test]
fn k8_4_a_max_is_only_for_cpu_and_gpu_use_with_a_mean_and_at_least_it() {
    let max = |column: usize, value: f64| {
        move |batch: &mut Batch| batch.points[0].max.as_mut().unwrap()[column] = Some(value)
    };
    assert_malformed("a disk max", max(1, 50.0));
    assert_malformed("below the mean", max(0, 12.0));
    assert_accepted("equal to the mean", max(0, 12.5));
    assert_malformed("without a mean", |batch| {
        batch.points[1].max = Some(vec![None, None, Some(4.0)]);
    });
    assert_malformed("a memory max", |batch| {
        batch.series[1] = crate::wire::BatchSeries {
            kind: "memory_bytes".into(),
            label: None,
            gpu: None,
        };
        batch.points[0].max.as_mut().unwrap()[1] = Some(50.0);
    });
}

#[test]
fn k8_4_every_number_is_finite_at_least_0_and_a_percent_at_most_100() {
    let mean = |column: usize, value: f64| {
        move |batch: &mut Batch| batch.points[1].mean[column] = Some(value)
    };
    for bad in [-1.0, -0.0, f64::NAN, f64::INFINITY, 100.000_001] {
        assert_malformed(&format!("cpu {bad}"), mean(0, bad));
        assert_malformed(&format!("disk {bad}"), mean(1, bad));
    }
    assert_accepted("100 percent", mean(0, 100.0));
    assert_accepted("0 percent", mean(0, 0.0));
    assert_malformed("a max over 100", |batch| {
        batch.points[0].max.as_mut().unwrap()[0] = Some(101.0);
    });
    assert_accepted("a large byte count", |batch| {
        batch.series[1] = crate::wire::BatchSeries {
            kind: "disk_bytes".into(),
            label: Some("/".into()),
            gpu: None,
        };
        batch.points[1].mean[1] = Some(1.5e19);
    });
}

fn gpu(index: u8, name: &str, vendor: GpuVendor, has_vram: bool) -> BatchGpu {
    BatchGpu {
        index,
        name: name.to_owned(),
        vendor,
        has_vram,
    }
}

#[test]
fn k8_4_gpus_have_unique_indexes_good_names_and_vram_only_of_their_own() {
    let gpus = |list: Vec<BatchGpu>| move |batch: &mut Batch| batch.gpus = Some(list);
    let nvidia = gpu(1, "RTX 4090", GpuVendor::Nvidia, true);
    assert_accepted("no gpus member", |batch| batch.gpus = None);
    assert_accepted("an empty list", gpus(Vec::new()));
    assert_accepted(
        "two GPUs",
        gpus(vec![gpu(0, "Arc", GpuVendor::Intel, false), nvidia.clone()]),
    );
    assert_accepted(
        "AMD without VRAM",
        gpus(vec![gpu(63, "Radeon 780M", GpuVendor::Amd, false)]),
    );
    assert_malformed("index 64", gpus(vec![gpu(64, "x", GpuVendor::Amd, true)]));
    assert_malformed(
        "an index twice",
        gpus(vec![gpu(1, "x", GpuVendor::Amd, true), nvidia]),
    );
    assert_malformed("no name", gpus(vec![gpu(0, "", GpuVendor::Amd, true)]));
    assert_malformed(
        "a long name",
        gpus(vec![gpu(0, &"x".repeat(129), GpuVendor::Amd, true)]),
    );
    assert_malformed(
        "a control character",
        gpus(vec![gpu(0, "x\r", GpuVendor::Amd, true)]),
    );
    assert_malformed(
        "Intel with VRAM",
        gpus(vec![gpu(0, "Arc", GpuVendor::Intel, true)]),
    );
    assert_malformed(
        "Apple with VRAM",
        gpus(vec![gpu(0, "M3", GpuVendor::Apple, true)]),
    );
}

#[test]
fn k8_4_an_unknown_vendor_is_not_a_batch() {
    let body = BATCH.replace("\"apple\"", "\"via\"");
    assert!(serde_json::from_str::<Batch>(&body).is_err());
}
