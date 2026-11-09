//! Which series a value belongs to (K8 Section 4).

use super::*;
use crate::points::SeriesKey;

const LABELLED: [Kind; 4] = [
    Kind::DiskPercent,
    Kind::DiskBytes,
    Kind::NetworkInBps,
    Kind::NetworkOutBps,
];
const PER_GPU: [Kind; 6] = [
    Kind::GpuPercent,
    Kind::VramPercent,
    Kind::VramBytes,
    Kind::GpuTemperatureC,
    Kind::GpuPowerW,
    Kind::GpuMemoryBytes,
];
const PLAIN: [Kind; 7] = [
    Kind::CpuPercent,
    Kind::MemoryPercent,
    Kind::MemoryBytes,
    Kind::UptimeSeconds,
    Kind::Load1,
    Kind::Load5,
    Kind::Load15,
];

fn key(kind: Kind, label: Option<&str>, gpu: Option<u8>) -> Option<SeriesKey> {
    SeriesKey::new(kind, label.map(str::to_owned), gpu)
}

#[test]
fn k8_a_label_is_given_exactly_for_the_disk_and_network_kinds() {
    for kind in LABELLED {
        assert!(key(kind, Some("/"), None).is_some(), "{kind:?}");
        assert!(key(kind, None, None).is_none(), "{kind:?}");
        assert!(key(kind, Some("/"), Some(0)).is_none(), "{kind:?}");
    }
    for kind in PLAIN.into_iter().chain(PER_GPU) {
        assert!(key(kind, Some("/"), None).is_none(), "{kind:?}");
    }
}

#[test]
fn k8_a_gpu_index_is_given_exactly_for_the_gpu_kinds_from_0_to_63() {
    for kind in PER_GPU {
        assert!(key(kind, None, Some(0)).is_some(), "{kind:?}");
        assert!(key(kind, None, Some(63)).is_some(), "{kind:?}");
        assert!(key(kind, None, Some(64)).is_none(), "{kind:?}");
        assert!(key(kind, None, None).is_none(), "{kind:?}");
    }
    for kind in PLAIN.into_iter().chain(LABELLED) {
        assert!(key(kind, None, Some(0)).is_none(), "{kind:?}");
    }
    for kind in PLAIN {
        assert!(key(kind, None, None).is_some(), "{kind:?}");
    }
}

#[test]
fn k8_a_label_is_1_to_128_bytes_without_a_control_character() {
    let longest = "é".repeat(64);
    assert_eq!(longest.len(), 128);
    assert!(key(Kind::DiskBytes, Some(&longest), None).is_some());
    let over = format!("{longest}x");
    for bad in ["", over.as_str(), "eth\u{0}0", "a\nb", "\u{7f}", "\u{9b}x"] {
        assert!(
            key(Kind::NetworkInBps, Some(bad), None).is_none(),
            "{bad:?}"
        );
    }
    assert!(key(Kind::DiskPercent, Some("/mnt/Backup Drive"), None).is_some());
}

#[test]
fn k8_the_key_of_a_collection_sample_drops_the_empty_label_of_a_plain_kind() {
    let cpu = Sample::new(Kind::CpuPercent, "", 5.0).unwrap();
    let disk = Sample::new(Kind::DiskPercent, "/", 5.0).unwrap();
    // Built by hand: `Sample::new` makes no GPU sample, so only a hand-made one lacks its index.
    let gpu = Sample {
        kind: Kind::GpuPercent,
        label: String::new(),
        value: 5.0,
        gpu_index: None,
    };
    let cpu_key = SeriesKey::of(&cpu).unwrap();
    assert_eq!(
        (cpu_key.kind(), cpu_key.label(), cpu_key.gpu()),
        (Kind::CpuPercent, None, None)
    );
    assert_eq!(SeriesKey::of(&disk).unwrap().label(), Some("/"));
    assert_eq!(SeriesKey::of(&gpu), None, "a GPU kind's sample with no GPU");
}

#[test]
fn r_099_the_key_of_a_gpu_sample_carries_its_index_and_no_label() {
    for kind in PER_GPU {
        let sample = Sample::gpu(kind, 3, 5.0).unwrap();
        let key = SeriesKey::of(&sample).unwrap();
        assert_eq!(
            (key.kind(), key.label(), key.gpu()),
            (kind, None, Some(3)),
            "{kind:?}"
        );
    }
    let first = SeriesKey::of(&Sample::gpu(Kind::GpuPercent, 0, 5.0).unwrap());
    let second = SeriesKey::of(&Sample::gpu(Kind::GpuPercent, 1, 5.0).unwrap());
    assert_ne!(first, second, "two GPUs share no series");
}
