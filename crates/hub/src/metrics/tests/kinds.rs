//! The K1 enums and the sample guard.

use async_graphql::EnumType;

use super::*;
use crate::metrics::{GpuVendor, MetricRange};

#[test]
fn k1_metric_kind_names_match_the_contract() {
    let names: Vec<&str> = MetricKind::items().iter().map(|item| item.name).collect();
    assert_eq!(
        names,
        [
            "CPU_PERCENT",
            "MEMORY_PERCENT",
            "MEMORY_BYTES",
            "DISK_PERCENT",
            "DISK_BYTES",
            "NETWORK_IN_BPS",
            "NETWORK_OUT_BPS",
            "UPTIME_SECONDS",
            "LOAD_1",
            "LOAD_5",
            "LOAD_15",
            "GPU_PERCENT",
            "VRAM_PERCENT",
            "VRAM_BYTES",
            "GPU_TEMPERATURE_C",
            "GPU_POWER_W",
        ]
    );
}

#[test]
fn k1_range_and_vendor_names_match_the_contract() {
    let ranges: Vec<&str> = MetricRange::items().iter().map(|item| item.name).collect();
    assert_eq!(ranges, ["HOUR", "DAY", "WEEK"]);
    let vendors: Vec<&str> = GpuVendor::items().iter().map(|item| item.name).collect();
    assert_eq!(vendors, ["NVIDIA", "AMD", "INTEL", "APPLE"]);
}

#[test]
fn every_kind_is_stored_as_lower_snake_case_and_read_back() {
    for item in MetricKind::items() {
        let stored = item.value.as_str();
        assert_eq!(stored, item.name.to_lowercase());
        assert_eq!(MetricKind::parse(stored), Some(item.value));
    }
}

#[test]
fn unknown_stored_kind_text_is_rejected() {
    for text in ["", "CPU_PERCENT", "cpu", "cpu_percent ", "load1"] {
        assert_eq!(MetricKind::parse(text), None, "{text:?}");
    }
}

#[test]
fn met_002_a_sample_is_a_finite_value_of_at_least_zero() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, -0.0] {
        assert_eq!(Sample::new(MetricKind::Load1, "", bad), None, "{bad}");
    }
    let sample = Sample::new(MetricKind::DiskBytes, "/", 1.5e19).unwrap();
    assert_eq!(sample.value.to_bits(), 1.5e19_f64.to_bits());
    assert_eq!(sample.label, "/");
}

#[test]
fn met_002_percent_kinds_are_clamped_to_100_and_others_are_not() {
    assert_eq!(
        Sample::new(MetricKind::CpuPercent, "", 250.0)
            .unwrap()
            .value
            .to_bits(),
        100.0_f64.to_bits()
    );
    assert_eq!(
        Sample::new(MetricKind::VramPercent, "", 100.5)
            .unwrap()
            .value
            .to_bits(),
        100.0_f64.to_bits()
    );
    assert_eq!(
        Sample::new(MetricKind::Load5, "", 250.0)
            .unwrap()
            .value
            .to_bits(),
        250.0_f64.to_bits()
    );
}
