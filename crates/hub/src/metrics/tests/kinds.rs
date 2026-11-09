//! The K1 enums and their stored names.

use agent_core::points::Kind;
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
            "GPU_MEMORY_BYTES",
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
fn k8_every_api_kind_is_a_stored_kind() {
    for item in MetricKind::items() {
        assert_eq!(MetricKind::served(Kind::from(item.value)), Some(item.value));
    }
}

#[test]
fn r_092_d_6_every_stored_kind_is_served_gpu_memory_bytes_too() {
    for kind in Kind::ALL {
        let served = MetricKind::served(kind);
        assert_eq!(served.map(Kind::from), Some(kind), "{kind:?}");
    }
    assert_eq!(
        MetricKind::parse("gpu_memory_bytes"),
        Some(MetricKind::GpuMemoryBytes)
    );
}
