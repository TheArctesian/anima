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
        Sample::gpu(Kind::VramPercent, 0, 100.5)
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

#[test]
fn met_009_a_gpu_sample_is_labelled_with_its_index_and_carries_it() {
    let sample = Sample::gpu(Kind::GpuPercent, 7, 250.0).unwrap();
    assert_eq!(sample.label, "7");
    assert_eq!(sample.gpu_index, Some(7));
    assert_eq!(sample.value.to_bits(), 100.0_f64.to_bits());
    let last = Sample::gpu(Kind::VramBytes, crate::points::MAX_GPU_INDEX, 1.0).unwrap();
    assert_eq!(last.label, "63");
    assert_eq!(Sample::new(Kind::Load1, "", 1.0).unwrap().gpu_index, None);
}

#[test]
fn r_099_a_gpu_kind_has_no_sample_without_its_index() {
    for kind in [
        Kind::GpuPercent,
        Kind::VramPercent,
        Kind::VramBytes,
        Kind::GpuTemperatureC,
        Kind::GpuPowerW,
        Kind::GpuMemoryBytes,
    ] {
        assert_eq!(Sample::new(kind, "", 1.0), None, "{kind:?}");
        assert_eq!(Sample::new(kind, "0", 1.0), None, "{kind:?}");
        assert!(Sample::gpu(kind, 0, 1.0).is_some(), "{kind:?}");
    }
}

#[test]
fn met_009_a_gpu_sample_needs_a_gpu_kind_an_index_k8_allows_and_a_kept_value() {
    assert_eq!(Sample::gpu(Kind::GpuPercent, 64, 1.0), None, "above 63");
    assert_eq!(Sample::gpu(Kind::GpuPowerW, u8::MAX, 1.0), None);
    assert_eq!(
        Sample::gpu(Kind::CpuPercent, 0, 1.0),
        None,
        "not a GPU kind"
    );
    assert_eq!(Sample::gpu(Kind::DiskBytes, 0, 1.0), None);
    assert_eq!(Sample::gpu(Kind::GpuTemperatureC, 0, f64::NAN), None);
    assert_eq!(Sample::gpu(Kind::GpuPowerW, 0, -1.0), None);
}

#[test]
fn met_002_a_gpu_temperature_or_power_beyond_its_ceiling_is_refused() {
    for (kind, ceiling, above) in [
        (Kind::GpuTemperatureC, 1000.0, 1000.5),
        (Kind::GpuPowerW, 10_000.0, 10_000.5),
    ] {
        let kept = Sample::gpu(kind, 0, ceiling).unwrap();
        assert_eq!(kept.value.to_bits(), f64::to_bits(ceiling), "{kind:?}");
        assert_eq!(Sample::gpu(kind, 0, above), None, "{kind:?}");
        assert_eq!(Sample::gpu(kind, 0, 1.7e308), None, "{kind:?}");
    }
    let bytes = Sample::gpu(Kind::VramBytes, 0, 1.7e19).unwrap();
    assert_eq!(bytes.value.to_bits(), 1.7e19_f64.to_bits(), "no ceiling");
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
