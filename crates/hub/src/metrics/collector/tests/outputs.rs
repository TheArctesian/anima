//! From command outputs to a reading and its samples: unreadable sources, partial
//! collections, the first reading and rates over the hub time (MET-002, D-4).

use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    metrics::{
        Cpu, Load, Memory, MetricKind, Reading, Sample,
        collector::outputs::{Parts, Previous, collected},
        commands::Source,
    },
    ssh::CommandOutput,
};
use agent_core::{
    points::Kind,
    readings::captures::{
        DF_LINUX, DF_MACOS, NETSTAT, PROC_LOADAVG, PROC_MEMINFO, PROC_NET_DEV, PROC_STAT,
        PROC_UPTIME, SYSCTL, TOP, VM_STAT,
    },
};

fn at() -> DateTime<Utc> {
    "2026-11-06T09:00:00Z".parse().unwrap()
}

/// The reading of `outputs`, each added in order as the collector does.
fn reading(outputs: &[(Source, CommandOutput)], at: DateTime<Utc>) -> Reading {
    let mut parts = Parts::default();
    for (source, output) in outputs {
        parts.add(*source, output);
    }
    parts.reading(at)
}

fn ok(stdout: &[u8]) -> CommandOutput {
    CommandOutput {
        exit_status: Some(0),
        stdout: stdout.to_vec(),
        cut: false,
    }
}

fn linux_outputs() -> Vec<(Source, CommandOutput)> {
    vec![
        (Source::Stat, ok(PROC_STAT)),
        (Source::Meminfo, ok(PROC_MEMINFO)),
        (Source::NetDev, ok(PROC_NET_DEV)),
        (Source::Uptime, ok(PROC_UPTIME)),
        (Source::Loadavg, ok(PROC_LOADAVG)),
        (Source::DiskBytes, ok(DF_LINUX)),
    ]
}

fn macos_outputs() -> Vec<(Source, CommandOutput)> {
    vec![
        (Source::Sysctl, ok(SYSCTL)),
        (Source::VmStat, ok(VM_STAT)),
        (Source::DiskKilobytes, ok(DF_MACOS)),
        (Source::Netstat, ok(NETSTAT)),
        (Source::Top, ok(TOP)),
    ]
}

/// The outputs with `source`'s replaced by `output`.
fn with(
    mut outputs: Vec<(Source, CommandOutput)>,
    source: Source,
    output: &CommandOutput,
) -> Vec<(Source, CommandOutput)> {
    for (from, kept) in &mut outputs {
        if *from == source {
            *kept = output.clone();
        }
    }
    outputs
}

fn value(samples: &[Sample], kind: MetricKind, label: &str) -> Option<f64> {
    samples
        .iter()
        .find(|sample| sample.kind == Kind::from(kind) && sample.label == label)
        .map(|sample| sample.value)
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("a sample");
    assert!(
        (actual - expected).abs() < 1e-6,
        "{actual} is not {expected}"
    );
}

#[test]
fn met_002_linux_outputs_give_every_field() {
    let read = reading(&linux_outputs(), at());
    assert_eq!(
        read.cpu,
        Some(Cpu::Ticks {
            idle: 3722,
            total: 9390
        })
    );
    assert_eq!(
        read.memory,
        Some(Memory {
            total_bytes: 16_777_216_000,
            used_bytes: 8_388_608_000,
        })
    );
    assert_eq!(
        read.load,
        Some(Load {
            one: 0.20,
            five: 0.18,
            fifteen: 0.12,
        })
    );
    assert_eq!(read.uptime_seconds, Some(350_735.47));
    assert_eq!(read.disks.len(), 7);
    assert_eq!(
        read.disks[2].total_bytes, 500_000_000_000,
        "blocks of 1 byte"
    );
    assert_eq!(read.interfaces.len(), 3);
}

#[test]
fn met_002_macos_outputs_give_every_field() {
    let read = reading(&macos_outputs(), at());
    let Some(Cpu::Percent(busy)) = read.cpu else {
        panic!("a busy share");
    };
    assert!((busy - 15.78).abs() < 1e-9, "{busy}");
    assert_eq!(
        read.memory,
        Some(Memory {
            total_bytes: 17_179_869_184,
            used_bytes: 5_798_100_992,
        })
    );
    assert_eq!(read.load.map(|load| load.five), Some(1.45));
    let boot = DateTime::<Utc>::from_timestamp(1_730_800_000, 0).unwrap();
    let since_boot = (at() - boot).num_seconds();
    assert_eq!(
        read.uptime_seconds,
        Some(f64::from(i32::try_from(since_boot).unwrap()))
    );
    assert_eq!(
        read.disks[0].total_bytes,
        500_000_000 * 1024,
        "blocks of 1024 bytes"
    );
    assert!(read.interfaces.iter().any(|one| one.name == "en0"));
}

#[test]
fn met_002_cut_output_leaves_its_source_out_and_the_others_count() {
    let mut cut = ok(PROC_MEMINFO);
    cut.cut = true;
    let read = reading(&with(linux_outputs(), Source::Meminfo, &cut), at());
    assert_eq!(read.memory, None);
    assert!(read.cpu.is_some());
    assert!(read.load.is_some());
    assert_eq!(read.disks.len(), 7);
}

#[test]
fn met_002_a_failed_or_missing_exit_status_leaves_its_source_out() {
    for exit_status in [Some(1), Some(127), None] {
        let failed = CommandOutput {
            exit_status,
            ..ok(DF_LINUX)
        };
        let read = reading(&with(linux_outputs(), Source::DiskBytes, &failed), at());
        assert!(read.disks.is_empty(), "{exit_status:?}");
        assert!(read.memory.is_some(), "{exit_status:?}");
    }
}

#[test]
fn met_002_macos_memory_needs_both_sysctl_and_vm_stat() {
    let failed = CommandOutput {
        exit_status: Some(1),
        ..ok(VM_STAT)
    };
    let read = reading(&with(macos_outputs(), Source::VmStat, &failed), at());
    assert_eq!(read.memory, None);
    assert!(read.load.is_some(), "sysctl still counts");
    let failed = CommandOutput {
        exit_status: Some(1),
        ..ok(SYSCTL)
    };
    let read = reading(&with(macos_outputs(), Source::Sysctl, &failed), at());
    assert_eq!(
        (read.memory, read.load, read.uptime_seconds),
        (None, None, None)
    );
    assert!(read.cpu.is_some(), "top still counts");
}

#[test]
fn met_002_no_outputs_give_an_empty_reading() {
    assert_eq!(reading(&[], at()), Reading::default());
}

#[test]
fn d4_the_first_linux_reading_has_no_cpu_or_network_sample() {
    let read = reading(&linux_outputs(), at());
    let out = collected(None, &read, at());
    assert_eq!(value(&out, MetricKind::CpuPercent, ""), None);
    assert!(!out.iter().any(|sample| sample.kind == Kind::NetworkInBps));
    assert_close(value(&out, MetricKind::MemoryPercent, ""), 50.0);
    assert_close(value(&out, MetricKind::DiskPercent, "/"), 25.0);
}

#[test]
fn d4_the_first_macos_reading_has_its_cpu_share() {
    let read = reading(&macos_outputs(), at());
    let out = collected(None, &read, at());
    assert_close(value(&out, MetricKind::CpuPercent, ""), 15.78);
}

/// The Linux reading 30 seconds later: 600 more ticks with 150 idle, eth0 3 MB in and 600 kB
/// out more.
fn later_linux() -> Reading {
    let stat = b"cpu  5155 356 584 3849 23 23 0 0 0 0\n";
    let net_dev = b"Inter-|   Receive\n face |bytes\n  \
eth0: 990654321 654321    0    0    0     0          0       100 124056789  234567    0    0    0     0       0          0\n";
    let outputs = with(
        with(linux_outputs(), Source::Stat, &ok(stat)),
        Source::NetDev,
        &ok(net_dev),
    );
    reading(&outputs, at() + TimeDelta::seconds(30))
}

#[test]
fn d4_the_second_reading_rates_cpu_and_network_over_the_hub_time_between() {
    let previous = Previous {
        at: at(),
        reading: reading(&linux_outputs(), at()),
    };
    let out = collected(
        Some(&previous),
        &later_linux(),
        at() + TimeDelta::seconds(30),
    );
    assert_close(value(&out, MetricKind::CpuPercent, ""), 75.0);
    assert_close(value(&out, MetricKind::NetworkInBps, "eth0"), 100_000.0);
    assert_close(value(&out, MetricKind::NetworkOutBps, "eth0"), 20_000.0);
    let slower = collected(
        Some(&previous),
        &later_linux(),
        at() + TimeDelta::seconds(60),
    );
    assert_close(value(&slower, MetricKind::NetworkInBps, "eth0"), 50_000.0);
}

#[test]
fn d4_a_hub_clock_that_stood_still_or_went_back_gives_no_rates() {
    let previous = Previous {
        at: at(),
        reading: reading(&linux_outputs(), at()),
    };
    for later in [at(), at() - TimeDelta::seconds(30)] {
        let out = collected(Some(&previous), &later_linux(), later);
        assert_eq!(value(&out, MetricKind::CpuPercent, ""), None, "{later}");
        assert_eq!(
            value(&out, MetricKind::NetworkInBps, "eth0"),
            None,
            "{later}"
        );
        assert!(value(&out, MetricKind::MemoryPercent, "").is_some());
    }
}
