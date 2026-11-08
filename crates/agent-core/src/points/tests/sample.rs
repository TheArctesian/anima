//! The sample guard and the kind names.

use super::*;

#[test]
fn met_002_a_sample_is_a_finite_value_of_at_least_zero() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, -0.0] {
        assert_eq!(Sample::new(Kind::Load1, "", bad), None, "{bad}");
    }
    let sample = Sample::new(Kind::DiskBytes, "/", 1.5e19).unwrap();
    assert_eq!(sample.value.to_bits(), 1.5e19_f64.to_bits());
    assert_eq!(sample.label, "/");
}

#[test]
fn met_002_percent_kinds_are_clamped_to_100_and_others_are_not() {
    assert_eq!(
        Sample::new(Kind::CpuPercent, "", 250.0)
            .unwrap()
            .value
            .to_bits(),
        100.0_f64.to_bits()
    );
    assert_eq!(
        Sample::new(Kind::VramPercent, "", 100.5)
            .unwrap()
            .value
            .to_bits(),
        100.0_f64.to_bits()
    );
    assert_eq!(
        Sample::new(Kind::Load5, "", 250.0).unwrap().value.to_bits(),
        250.0_f64.to_bits()
    );
}

const STORED: [&str; 17] = [
    "cpu_percent",
    "memory_percent",
    "memory_bytes",
    "disk_percent",
    "disk_bytes",
    "network_in_bps",
    "network_out_bps",
    "uptime_seconds",
    "load_1",
    "load_5",
    "load_15",
    "gpu_percent",
    "vram_percent",
    "vram_bytes",
    "gpu_temperature_c",
    "gpu_power_w",
    "gpu_memory_bytes",
];

#[test]
fn k8_every_series_kind_of_the_contract_is_read_and_written_back() {
    for text in STORED {
        assert_eq!(Kind::parse(text).map(Kind::as_str), Some(text));
    }
}

#[test]
fn k8_unknown_kind_text_is_rejected() {
    for text in [
        "",
        "CPU_PERCENT",
        "cpu",
        "cpu_percent ",
        "load1",
        "gpu_memory",
    ] {
        assert_eq!(Kind::parse(text), None, "{text:?}");
    }
}

#[test]
fn met_007_gpu_memory_bytes_is_a_gpu_kind_and_never_a_percent_or_a_max() {
    let kind = Kind::GpuMemoryBytes;
    assert!(kind.is_per_gpu());
    assert!(!kind.is_percent());
    assert!(!kind.has_max());
    assert!(!kind.is_labelled());
}

#[test]
fn met_014_only_cpu_and_gpu_use_keep_a_max() {
    let with_max: Vec<&str> = STORED
        .into_iter()
        .filter(|text| Kind::parse(text).unwrap().has_max())
        .collect();
    assert_eq!(with_max, ["cpu_percent", "gpu_percent"]);
}
