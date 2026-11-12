//! The device and time refusals, their order, and what the hub takes (K8 Sections 4 and 6,
//! MET-017).

use std::collections::BTreeMap;

use chrono::TimeDelta;

use super::*;
use crate::points::{Kind, Point, SeriesKey, Summary};

fn checked_at(now: DateTime<Utc>) -> Result<Accepted, RefusalCode> {
    example().check(device(), now)
}

#[test]
fn met_017_a_batch_for_another_device_is_refused_before_its_shape() {
    let other: Uuid = "0192f5c4-6f1e-7a3b-9c2d-5e8f1a2b3c4e".parse().unwrap();
    assert_eq!(
        example().check(other, now()).err(),
        Some(RefusalCode::WrongDevice)
    );
    let mut broken = example();
    broken.series.clear();
    assert_eq!(
        broken.check(other, now()).err(),
        Some(RefusalCode::WrongDevice)
    );
}

#[test]
fn met_017_the_shape_is_refused_before_the_times() {
    let mut broken = example();
    broken.points[0].mean.clear();
    let late = now() + TimeDelta::days(1);
    assert_eq!(
        broken.check(device(), late).err(),
        Some(RefusalCode::Malformed)
    );
}

#[test]
fn met_017_a_point_may_be_exactly_1_hour_old_but_not_a_second_more() {
    let oldest = at("2026-11-08T10:00:10Z");
    assert!(checked_at(oldest + TimeDelta::hours(1)).is_ok());
    let later = oldest + TimeDelta::hours(1) + TimeDelta::seconds(1);
    assert_eq!(checked_at(later).err(), Some(RefusalCode::OutOfRange));
}

#[test]
fn met_017_a_point_may_be_exactly_60_seconds_ahead_but_not_a_second_more() {
    let newest = at("2026-11-08T10:00:20Z");
    assert!(checked_at(newest - TimeDelta::seconds(60)).is_ok());
    let earlier = newest - TimeDelta::seconds(61);
    assert_eq!(checked_at(earlier).err(), Some(RefusalCode::OutOfRange));
}

fn key(kind: Kind, label: Option<&str>, gpu: Option<u8>) -> SeriesKey {
    SeriesKey::new(kind, label.map(str::to_owned), gpu).unwrap()
}

fn summary(mean: f64, max: Option<f64>) -> Summary {
    Summary { mean, max }
}

#[test]
fn met_015_the_hub_takes_each_non_null_mean_with_its_max() {
    let accepted = example().check(device(), now()).unwrap();
    let cpu = key(Kind::CpuPercent, None, None);
    let disk = key(Kind::DiskPercent, Some("/"), None);
    let busy = key(Kind::GpuPercent, None, Some(0));
    let first = BTreeMap::from([
        (cpu.clone(), summary(12.5, Some(31.0))),
        (disk.clone(), summary(40.1, None)),
        (busy, summary(3.0, Some(9.5))),
    ]);
    let second = BTreeMap::from([(cpu, summary(11.0, None)), (disk, summary(40.1, None))]);
    assert_eq!(
        accepted.points,
        [
            Point::new(at("2026-11-08T10:00:10Z"), first),
            Point::new(at("2026-11-08T10:00:20Z"), second),
        ]
    );
    assert_eq!(accepted.gpus, example().gpus);
}

#[test]
fn met_014_a_batch_the_agent_makes_is_one_the_hub_takes_back_unchanged() {
    let cpu = key(Kind::CpuPercent, None, None);
    let memory = key(Kind::GpuMemoryBytes, None, Some(0));
    let points = [
        Point::new(
            at("2026-11-08T10:00:10Z"),
            BTreeMap::from([(memory.clone(), summary(8.0e9, None))]),
        ),
        Point::new(
            at("2026-11-08T10:00:20Z"),
            BTreeMap::from([
                (cpu, summary(5.0, Some(7.0))),
                (memory, summary(8.5e9, None)),
            ]),
        ),
    ];
    let batch = Batch::new(device(), &points, (None, None));
    let body = serde_json::to_value(&batch).unwrap();
    assert_eq!(
        body["series"][0],
        serde_json::json!({ "kind": "cpu_percent" })
    );
    assert_eq!(body["points"][0]["mean"], serde_json::json!([null, 8.0e9]));
    assert!(
        body["points"][0].get("max").is_none(),
        "no max in a point without one"
    );
    assert!(body.get("gpus").is_none());
    let read: Batch = serde_json::from_value(body).unwrap();
    assert_eq!(read.check(device(), now()).unwrap().points, points);
}

/// What the hub takes of one GPU column of `kind` whose mean is `value`, or its refusal.
fn taken_gpu_value(kind: &str, value: f64) -> Result<Option<f64>, RefusalCode> {
    let mut batch = example();
    batch.series = vec![crate::wire::BatchSeries {
        kind: kind.to_owned(),
        label: None,
        gpu: Some(0),
    }];
    batch.points.truncate(1);
    batch.points[0].mean = vec![Some(value)];
    batch.points[0].max = None;
    let accepted = batch.check(device(), now())?;
    let point = accepted.points.first().unwrap();
    Ok(point.values().values().next().map(|summary| summary.mean))
}

#[test]
fn cr_17_a_pushed_gpu_temperature_or_power_past_its_ceiling_is_malformed() {
    for (kind, ceiling) in [("gpu_temperature_c", 1000.0), ("gpu_power_w", 10_000.0)] {
        assert_eq!(taken_gpu_value(kind, ceiling), Ok(Some(ceiling)), "{kind}");
        let above = f64::from_bits(f64::to_bits(ceiling) + 1);
        assert_eq!(
            taken_gpu_value(kind, above),
            Err(RefusalCode::Malformed),
            "{kind}"
        );
        assert_eq!(
            taken_gpu_value(kind, 1.7e308),
            Err(RefusalCode::Malformed),
            "{kind}"
        );
    }
    assert_eq!(
        taken_gpu_value("vram_bytes", 1.7e308),
        Ok(Some(1.7e308)),
        "VRAM bytes have no ceiling"
    );
}
