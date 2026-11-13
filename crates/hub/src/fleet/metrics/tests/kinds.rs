//! Storage and agent mappings for metric kinds.

use agent_core::points::Kind;
use async_graphql::EnumType;

use super::*;

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
