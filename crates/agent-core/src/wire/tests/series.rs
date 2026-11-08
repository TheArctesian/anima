//! The series rules of a batch (K8 Section 4): each refuses its own case only.

use super::*;
use crate::wire::BatchSeries;

fn series(kind: &str, label: Option<&str>, gpu: Option<u8>) -> BatchSeries {
    BatchSeries {
        kind: kind.to_owned(),
        label: label.map(str::to_owned),
        gpu,
    }
}

/// Adds `added` as a last column, with `value` as its mean in every point.
fn with_series(batch: &mut Batch, added: BatchSeries, value: Option<f64>) {
    batch.series.push(added);
    for point in &mut batch.points {
        point.mean.push(value);
        if let Some(max) = &mut point.max {
            max.push(None);
        }
    }
}

/// Keeps only the first `count` columns.
fn keep_columns(batch: &mut Batch, count: usize) {
    batch.series.truncate(count);
    for point in &mut batch.points {
        point.mean.truncate(count);
        if let Some(max) = &mut point.max {
            max.truncate(count);
        }
    }
}

#[test]
fn k8_4_a_batch_has_1_to_256_series() {
    assert_malformed("no series", |batch| keep_columns(batch, 0));
    assert_accepted("one series", |batch| keep_columns(batch, 1));
    let disks = |count: usize| {
        move |batch: &mut Batch| {
            for n in 0..count - batch.series.len() {
                with_series(
                    batch,
                    series("disk_bytes", Some(&format!("/m{n}")), None),
                    None,
                );
            }
        }
    };
    assert_accepted("256 series", disks(256));
    assert_malformed("257 series", disks(257));
}

#[test]
fn k8_4_no_two_series_are_equal() {
    assert_malformed("cpu twice", |batch| {
        with_series(batch, series("cpu_percent", None, None), Some(1.0));
    });
    assert_malformed("an unknown kind twice", |batch| {
        with_series(batch, series("fan_rpm", None, None), None);
        with_series(batch, series("fan_rpm", None, None), None);
    });
    assert_accepted("another mount", |batch| {
        with_series(
            batch,
            series("disk_percent", Some("/home"), None),
            Some(1.0),
        );
    });
    assert_accepted("another GPU", |batch| {
        with_series(batch, series("gpu_percent", None, Some(1)), Some(1.0));
    });
}

#[test]
fn k8_4_a_label_is_present_exactly_for_disk_and_network_kinds() {
    assert_malformed("disk without a label", |batch| batch.series[1].label = None);
    assert_malformed("cpu with a label", |batch| {
        batch.series[0].label = Some("x".into());
    });
    assert_malformed("gpu with a label", |batch| {
        batch.series[2].label = Some("x".into());
    });
    assert_malformed("network without a label", |batch| {
        with_series(batch, series("network_in_bps", None, None), Some(1.0));
    });
    assert_accepted("network with a label", |batch| {
        with_series(
            batch,
            series("network_in_bps", Some("en0"), None),
            Some(1.0),
        );
    });
}

#[test]
fn k8_4_a_label_is_1_to_128_bytes_without_a_control_character() {
    let label = |text: String| move |batch: &mut Batch| batch.series[1].label = Some(text);
    assert_accepted("128 bytes", label("é".repeat(64)));
    assert_malformed("129 bytes", label(format!("{}x", "é".repeat(64))));
    assert_malformed("empty", label(String::new()));
    assert_malformed("a newline", label("/mnt\n".into()));
    assert_malformed("an escape", label("/\u{1b}[31m".into()));
    assert_malformed("a C1 control", label("/\u{85}".into()));
    assert_accepted("a space and unicode", label("/mnt/Backup Drive ✓".into()));
}

#[test]
fn k8_4_a_gpu_index_0_to_63_is_present_exactly_for_gpu_kinds() {
    assert_malformed("gpu kind without an index", |batch| {
        batch.series[2].gpu = None;
    });
    assert_malformed("index 64", |batch| batch.series[2].gpu = Some(64));
    assert_accepted("index 63", |batch| batch.series[2].gpu = Some(63));
    assert_malformed("cpu with an index", |batch| batch.series[0].gpu = Some(0));
    assert_malformed("disk with an index", |batch| batch.series[1].gpu = Some(0));
    for kind in [
        "vram_percent",
        "vram_bytes",
        "gpu_temperature_c",
        "gpu_power_w",
        "gpu_memory_bytes",
    ] {
        assert_malformed(kind, |batch| {
            with_series(batch, series(kind, None, None), None);
        });
        assert_accepted(kind, |batch| {
            with_series(batch, series(kind, None, Some(0)), Some(1.0));
        });
    }
}

#[test]
fn k8_4_a_series_of_an_unknown_kind_is_ignored_with_its_column() {
    let accepted = checked(|batch| {
        let odd = series("fan_rpm", Some("\u{7}"), Some(200));
        with_series(batch, odd, Some(-5.0));
        for point in &mut batch.points {
            point.max = Some(vec![None, None, None, Some(-9.0)]);
        }
    })
    .unwrap();
    assert_eq!(accepted.points.len(), 2);
    assert!(accepted.points.iter().all(|point| {
        point
            .values()
            .keys()
            .all(|key| key.kind().as_str() != "fan_rpm")
    }));
    assert_eq!(accepted.points[0].values().len(), 3);
}
