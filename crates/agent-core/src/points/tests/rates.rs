//! From readings to samples (MET-002, D-4, D-5).

use std::time::Duration;

use super::*;
use crate::{
    points::samples,
    readings::{captures::*, df, linux, macos},
};

pub const THIRTY: Duration = Duration::from_secs(30);

pub fn linux_reading() -> Reading {
    Reading {
        cpu: linux::cpu(PROC_STAT),
        memory: linux::memory(PROC_MEMINFO),
        load: linux::load_average(PROC_LOADAVG),
        uptime_seconds: linux::uptime_seconds(PROC_UPTIME),
        disks: df::disks(DF_LINUX, 1),
        interfaces: linux::interfaces(PROC_NET_DEV),
    }
}

#[test]
fn met_002_the_first_linux_reading_gives_no_cpu_or_network_sample_but_the_rest() {
    let out = samples(None, &linux_reading(), THIRTY);
    assert_eq!(count(&out, Kind::CpuPercent), 0);
    assert_eq!(count(&out, Kind::NetworkInBps), 0);
    assert_eq!(count(&out, Kind::NetworkOutBps), 0);
    assert_close(value(&out, Kind::MemoryPercent, "").unwrap(), 50.0);
    assert_close(value(&out, Kind::MemoryBytes, "").unwrap(), 8_388_608_000.0);
    assert_close(value(&out, Kind::UptimeSeconds, "").unwrap(), 350_735.47);
    assert_close(value(&out, Kind::Load1, "").unwrap(), 0.20);
    assert_close(value(&out, Kind::Load5, "").unwrap(), 0.18);
    assert_close(value(&out, Kind::Load15, "").unwrap(), 0.12);
}

#[test]
fn met_002_disk_samples_are_percent_and_used_bytes_per_real_mount() {
    let out = samples(None, &linux_reading(), THIRTY);
    assert_eq!(count(&out, Kind::DiskPercent), 3);
    assert_eq!(count(&out, Kind::DiskBytes), 3);
    assert_close(value(&out, Kind::DiskPercent, "/").unwrap(), 25.0);
    assert_close(
        value(&out, Kind::DiskBytes, "/").unwrap(),
        125_000_000_000.0,
    );
    assert_close(value(&out, Kind::DiskPercent, "/boot/efi").unwrap(), 50.0);
    assert_close(
        value(&out, Kind::DiskPercent, "/mnt/Backup Drive").unwrap(),
        10.0,
    );
}

#[test]
fn met_002_a_disk_with_no_blocks_is_skipped_and_used_over_total_is_held_to_100() {
    let disks = vec![disk("x", "/zero", 0, 0), disk("x", "/over", 100, 300)];
    let out = samples(
        None,
        &Reading {
            disks,
            ..Reading::default()
        },
        THIRTY,
    );
    assert_eq!(count(&out, Kind::DiskPercent), 1);
    assert_close(value(&out, Kind::DiskPercent, "/over").unwrap(), 100.0);
    assert_close(value(&out, Kind::DiskBytes, "/over").unwrap(), 100.0);
}

#[test]
fn met_002_cpu_percent_is_the_busy_share_of_the_jiffies_since_the_last_reading() {
    let before = linux_reading();
    let mut now = linux_reading();
    // 230 jiffies passed, 110 of them idle.
    now.cpu = Some(Cpu::Ticks {
        idle: 3832,
        total: 9620,
    });
    let out = samples(Some(&before), &now, THIRTY);
    assert_close(
        value(&out, Kind::CpuPercent, "").unwrap(),
        120.0 / 230.0 * 100.0,
    );
}

#[test]
fn met_002_d4_cpu_counters_that_went_back_or_stood_still_give_no_cpu_sample() {
    let before = linux_reading();
    for cpu in [
        Cpu::Ticks {
            idle: 100,
            total: 5000,
        },
        Cpu::Ticks {
            idle: 3700,
            total: 9400,
        },
        Cpu::Ticks {
            idle: 3722,
            total: 9390,
        },
        Cpu::Ticks {
            idle: 4000,
            total: 9400,
        },
    ] {
        let now = Reading {
            cpu: Some(cpu),
            ..linux_reading()
        };
        let out = samples(Some(&before), &now, THIRTY);
        assert_eq!(count(&out, Kind::CpuPercent), 0, "{cpu:?}");
    }
}

#[test]
fn met_002_d4_no_time_elapsed_gives_no_cpu_or_network_sample() {
    let before = linux_reading();
    let mut now = linux_reading();
    now.cpu = Some(Cpu::Ticks {
        idle: 3832,
        total: 9620,
    });
    now.interfaces = vec![interface("eth0", 987_654_321 + 3_000_000, 123_456_789)];
    let out = samples(Some(&before), &now, Duration::ZERO);
    assert_eq!(count(&out, Kind::CpuPercent), 0);
    assert_eq!(count(&out, Kind::NetworkInBps), 0);
    assert_eq!(count(&out, Kind::MemoryBytes), 1);
}

#[test]
fn met_002_cpu_percent_is_100_when_no_jiffy_was_idle() {
    let before = Reading {
        cpu: Some(Cpu::Ticks {
            idle: 10,
            total: 100,
        }),
        ..Reading::default()
    };
    let now = Reading {
        cpu: Some(Cpu::Ticks {
            idle: 10,
            total: 200,
        }),
        ..Reading::default()
    };
    let out = samples(Some(&before), &now, THIRTY);
    assert_close(value(&out, Kind::CpuPercent, "").unwrap(), 100.0);
}

#[test]
fn met_002_memory_with_a_zero_total_or_more_used_than_total_is_safe() {
    let zero = Reading {
        memory: Some(Memory {
            total_bytes: 0,
            used_bytes: 0,
        }),
        ..Reading::default()
    };
    assert_eq!(samples(None, &zero, THIRTY).len(), 0);
    let over = Reading {
        memory: Some(Memory {
            total_bytes: 100,
            used_bytes: 400,
        }),
        ..Reading::default()
    };
    let out = samples(None, &over, THIRTY);
    assert_close(value(&out, Kind::MemoryPercent, "").unwrap(), 100.0);
    assert_close(value(&out, Kind::MemoryBytes, "").unwrap(), 100.0);
}

#[test]
fn met_002_a_macos_reading_gives_its_busy_share_on_the_first_collection() {
    let sysctl = macos::sysctl(SYSCTL);
    let now = chrono::DateTime::from_timestamp(1_730_803_600, 0).unwrap();
    let reading = Reading {
        cpu: macos::cpu(TOP),
        memory: Some(Memory {
            total_bytes: sysctl.memory_bytes.unwrap(),
            used_bytes: macos::memory_used_bytes(VM_STAT, sysctl.page_size).unwrap(),
        }),
        load: sysctl.load,
        uptime_seconds: sysctl.uptime_seconds(now),
        disks: df::disks(DF_MACOS, 1024),
        interfaces: macos::interfaces(NETSTAT),
    };
    let out = samples(None, &reading, THIRTY);
    assert_close(value(&out, Kind::CpuPercent, "").unwrap(), 15.78);
    assert_close(value(&out, Kind::MemoryBytes, "").unwrap(), 5_798_100_992.0);
    assert_close(
        value(&out, Kind::MemoryPercent, "").unwrap(),
        5_798_100_992.0 / 17_179_869_184.0 * 100.0,
    );
    assert_close(value(&out, Kind::UptimeSeconds, "").unwrap(), 3600.0);
    assert_close(value(&out, Kind::Load15, "").unwrap(), 1.67);
    assert_close(value(&out, Kind::DiskPercent, "/").unwrap(), 25.0);
    assert_close(
        value(&out, Kind::DiskPercent, "/System/Volumes/Data").unwrap(),
        50.0,
    );
    assert_eq!(count(&out, Kind::DiskPercent), 2);
}

#[test]
fn met_002_non_finite_or_negative_plain_values_are_left_out() {
    let reading = Reading {
        uptime_seconds: Some(f64::NAN),
        load: Some(Load {
            one: f64::INFINITY,
            five: -1.0,
            fifteen: 2.0,
        }),
        cpu: Some(Cpu::Percent(f64::NAN)),
        ..Reading::default()
    };
    let out = samples(None, &reading, THIRTY);
    assert_eq!(out.len(), 1);
    assert_close(value(&out, Kind::Load15, "").unwrap(), 2.0);
}

#[test]
fn met_002_a_wholly_unreadable_device_gives_no_samples() {
    let reading = Reading {
        cpu: linux::cpu(b"\xff\xfe"),
        memory: linux::memory(b""),
        load: linux::load_average(b"x"),
        uptime_seconds: linux::uptime_seconds(b"x"),
        disks: df::disks(b"\x00\x01", 1),
        interfaces: linux::interfaces(b"nothing here"),
    };
    assert_eq!(reading, Reading::default());
    assert_eq!(samples(Some(&reading), &reading, THIRTY).len(), 0);
}
