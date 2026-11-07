//! Which disks and interfaces become series, and the network rates (MET-002, D-4, D-5).

use std::time::Duration;

use super::{
    rates::{THIRTY, linux_reading},
    *,
};
use crate::metrics::{MAX_LABELS, samples};

#[test]
fn met_002_d5_pseudo_file_systems_and_mounts_are_left_out() {
    let disks = vec![
        disk("tmpfs", "/mnt/ram", 100, 50),
        disk("devtmpfs", "/mnt/dt", 100, 50),
        disk("overlay", "/mnt/ov", 100, 50),
        disk("squashfs", "/snap/core", 100, 50),
        disk("devfs", "/mnt/devfs", 100, 50),
        disk("map", "/mnt/map", 100, 50),
        disk("x", "/dev", 100, 50),
        disk("x", "/dev/shm", 100, 50),
        disk("x", "/run/user/1000", 100, 50),
        disk("x", "/sys/fs/cgroup", 100, 50),
        disk("x", "/proc", 100, 50),
        disk("x", "/System/Volumes/VM", 100, 50),
        disk("x", "/System/Volumes/Data/home", 100, 50),
        disk("x", "/System/Volumes/Data", 100, 50),
        disk("x", "/devices", 100, 25),
        disk("x", "/runner", 100, 75),
    ];
    let out = samples(
        None,
        &Reading {
            disks,
            ..Reading::default()
        },
        THIRTY,
    );
    let mut mounts: Vec<&str> = out
        .iter()
        .filter(|sample| sample.kind == MetricKind::DiskPercent)
        .map(|sample| sample.label.as_str())
        .collect();
    mounts.sort_unstable();
    assert_eq!(mounts, ["/System/Volumes/Data", "/devices", "/runner"]);
}

#[test]
fn met_002_network_rates_are_byte_deltas_over_seconds_per_interface_without_loopback() {
    let before = linux_reading();
    let mut now = linux_reading();
    now.interfaces = vec![
        interface("lo", 2_000_000, 2_000_000),
        interface("eth0", 987_654_321 + 3_000_000, 123_456_789 + 600_000),
        interface("docker0", 5000, 9000 + 300),
    ];
    let out = samples(Some(&before), &now, THIRTY);
    assert_eq!(count(&out, MetricKind::NetworkInBps), 2);
    assert_close(
        value(&out, MetricKind::NetworkInBps, "eth0").unwrap(),
        100_000.0,
    );
    assert_close(
        value(&out, MetricKind::NetworkOutBps, "eth0").unwrap(),
        20_000.0,
    );
    assert_close(
        value(&out, MetricKind::NetworkInBps, "docker0").unwrap(),
        0.0,
    );
    assert_close(
        value(&out, MetricKind::NetworkOutBps, "docker0").unwrap(),
        10.0,
    );
    assert_eq!(value(&out, MetricKind::NetworkInBps, "lo"), None);
    assert_eq!(value(&out, MetricKind::NetworkInBps, "lo0"), None);
}

#[test]
fn met_002_d4_an_interface_whose_counter_went_back_or_is_new_gives_no_network_sample() {
    let before = Reading {
        interfaces: vec![
            interface("eth0", 1000, 1000),
            interface("eth1", 1000, 1000),
            interface("eth2", 0, 0),
        ],
        ..Reading::default()
    };
    let now = Reading {
        interfaces: vec![
            interface("eth0", 500, 2000),
            interface("eth1", 2000, 500),
            interface("eth2", 10, 20),
            interface("eth3", 10, 20),
        ],
        ..Reading::default()
    };
    let out = samples(Some(&before), &now, Duration::from_secs(10));
    assert_eq!(count(&out, MetricKind::NetworkInBps), 1);
    assert_close(value(&out, MetricKind::NetworkInBps, "eth2").unwrap(), 1.0);
    assert_close(value(&out, MetricKind::NetworkOutBps, "eth2").unwrap(), 2.0);
}

#[test]
fn met_002_network_counters_at_u64_max_give_finite_rates() {
    let before = Reading {
        interfaces: vec![interface("eth0", 0, 0)],
        ..Reading::default()
    };
    let now = Reading {
        interfaces: vec![interface("eth0", u64::MAX, u64::MAX)],
        ..Reading::default()
    };
    let out = samples(Some(&before), &now, Duration::from_secs(1));
    assert_close(
        value(&out, MetricKind::NetworkInBps, "eth0").unwrap() / 1e19,
        1.844_674_407_370_955_2e19 / 1e19,
    );
}

#[test]
fn met_002_d5_at_most_32_network_and_32_disk_series_the_first_32_by_name() {
    let names: Vec<String> = (0..40).rev().map(|n| format!("veth{n:02}")).collect();
    let before = Reading {
        interfaces: names.iter().map(|name| interface(name, 0, 0)).collect(),
        ..Reading::default()
    };
    let now = Reading {
        interfaces: names.iter().map(|name| interface(name, 100, 200)).collect(),
        disks: (0..40)
            .rev()
            .map(|n| disk("ext4", &format!("/m{n:02}"), 100, 10))
            .collect(),
        ..Reading::default()
    };
    let out = samples(Some(&before), &now, Duration::from_secs(1));
    assert_eq!(MAX_LABELS, 32);
    assert_eq!(count(&out, MetricKind::NetworkInBps), 32);
    assert_eq!(count(&out, MetricKind::NetworkOutBps), 32);
    assert_eq!(count(&out, MetricKind::DiskPercent), 32);
    assert!(value(&out, MetricKind::NetworkInBps, "veth31").is_some());
    assert!(value(&out, MetricKind::NetworkInBps, "veth32").is_none());
    assert!(value(&out, MetricKind::DiskBytes, "/m31").is_some());
    assert!(value(&out, MetricKind::DiskBytes, "/m32").is_none());
}

#[test]
fn met_002_d5_a_repeated_mount_or_interface_name_makes_one_series() {
    let reading = Reading {
        interfaces: vec![interface("en0", 10, 10), interface("en0", 99, 99)],
        disks: vec![disk("a", "/", 100, 10), disk("b", "/", 100, 90)],
        ..Reading::default()
    };
    let later = Reading {
        interfaces: vec![interface("en0", 20, 30)],
        ..reading.clone()
    };
    let out = samples(Some(&reading), &later, Duration::from_secs(1));
    assert_eq!(count(&out, MetricKind::DiskPercent), 1);
    assert_close(value(&out, MetricKind::DiskPercent, "/").unwrap(), 10.0);
    assert_close(value(&out, MetricKind::NetworkInBps, "en0").unwrap(), 10.0);
    assert_close(value(&out, MetricKind::NetworkOutBps, "en0").unwrap(), 20.0);
}
