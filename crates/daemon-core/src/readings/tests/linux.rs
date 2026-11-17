//! The Linux `/proc` readers (MET-002).

use super::{captures::*, *};
use crate::readings::linux;

#[test]
fn met_002_proc_stat_gives_idle_and_total_jiffies_of_the_aggregate_line() {
    // total: 4705 + 356 + 584 + 3699 + 23 + 23; idle: 3699 + 23.
    assert_eq!(
        linux::cpu(PROC_STAT),
        Some(Cpu::Ticks {
            idle: 3722,
            total: 9390
        })
    );
}

#[test]
fn met_002_proc_stat_with_four_fields_is_read_and_with_three_is_not() {
    assert_eq!(
        linux::cpu(b"cpu 10 0 5 85\n"),
        Some(Cpu::Ticks {
            idle: 85,
            total: 100
        })
    );
    assert_eq!(linux::cpu(b"cpu 10 0 5\n"), None);
}

#[test]
fn met_002_proc_stat_with_a_bad_field_empty_or_overflowing_gives_none() {
    assert_eq!(linux::cpu(b"cpu  1 2 x 4 5\n"), None);
    assert_eq!(linux::cpu(b"cpu  1 2 3 -4 5\n"), None);
    assert_eq!(linux::cpu(b""), None);
    assert_eq!(linux::cpu(b"cpu0 1 2 3 4 5\n"), None);
    let huge = format!("cpu {m} {m} 0 0\n", m = u64::MAX);
    assert_eq!(linux::cpu(huge.as_bytes()), None);
}

#[test]
fn met_002_meminfo_uses_available_memory() {
    assert_eq!(
        linux::memory(PROC_MEMINFO),
        Some(Memory {
            total_bytes: 16_777_216_000,
            used_bytes: 8_388_608_000
        })
    );
}

#[test]
fn met_002_meminfo_without_available_falls_back_to_free_buffers_and_cached() {
    // Used: 16384000 - (1024000 + 512000 + 4096000) kB.
    assert_eq!(
        linux::memory(PROC_MEMINFO_OLD),
        Some(Memory {
            total_bytes: 16_777_216_000,
            used_bytes: 11_010_048_000
        })
    );
}

#[test]
fn met_002_meminfo_without_what_it_needs_gives_none() {
    assert_eq!(linux::memory(b"MemFree: 1 kB\nMemAvailable: 1 kB\n"), None);
    assert_eq!(linux::memory(b"MemTotal: 0 kB\nMemAvailable: 0 kB\n"), None);
    assert_eq!(
        linux::memory(b"MemTotal: 100 kB\nMemFree: 1 kB\nBuffers: 1 kB\n"),
        None
    );
    assert_eq!(linux::memory(b""), None);
}

#[test]
fn met_002_meminfo_cut_in_a_line_drops_the_cut_line() {
    let cut = b"MemTotal:       16384000 kB\nMemAvailable:   8192";
    assert_eq!(linux::memory(cut), None);
    let cut_after = b"MemTotal:       16384000 kB\nMemAvailable:   8192000 kB\nMemFree:  10";
    assert_eq!(
        linux::memory(cut_after),
        Some(Memory {
            total_bytes: 16_777_216_000,
            used_bytes: 8_388_608_000
        })
    );
}

#[test]
fn met_002_meminfo_that_overflows_bytes_or_reports_more_available_than_total() {
    assert_eq!(
        linux::memory(b"MemTotal: 18446744073709551615 kB\nMemAvailable: 1 kB\n"),
        None
    );
    assert_eq!(
        linux::memory(b"MemTotal: 100 kB\nMemAvailable: 200 kB\n"),
        Some(Memory {
            total_bytes: 102_400,
            used_bytes: 0
        })
    );
}

#[test]
fn met_002_net_dev_gives_received_and_sent_bytes_per_interface() {
    assert_eq!(
        linux::interfaces(PROC_NET_DEV),
        [
            interface("lo", 1_000_000, 1_000_000),
            interface("eth0", 987_654_321, 123_456_789),
            interface("docker0", 5000, 9000),
        ]
    );
}

#[test]
fn met_002_net_dev_skips_bad_lines_and_keeps_the_good_ones() {
    let output =
        b"junk\n eth0: 1 2 3\n eth1: x 0 0 0 0 0 0 0 5 0\n eth2: 7 0 0 0 0 0 0 0 9 0 0 0 0 0 0 0\n\
 eth3: 18446744073709551616 0 0 0 0 0 0 0 9 0\n eth4: 18446744073709551615 0 0 0 0 0 0 0 0 0\n";
    assert_eq!(
        linux::interfaces(output),
        [interface("eth2", 7, 9), interface("eth4", u64::MAX, 0),]
    );
}

#[test]
fn met_002_net_dev_skips_a_name_that_is_not_utf8_too_long_or_has_a_control_character() {
    let long = "a".repeat(129);
    let edge = "b".repeat(128);
    let mut output = Vec::new();
    output.extend_from_slice(b" bad\xff: 1 0 0 0 0 0 0 0 2 0\n");
    output.extend_from_slice(format!("{long}: 1 0 0 0 0 0 0 0 2 0\n").as_bytes());
    output.extend_from_slice(b"ab\x07c: 1 0 0 0 0 0 0 0 2 0\n");
    output.extend_from_slice(b": 1 0 0 0 0 0 0 0 2 0\n");
    output.extend_from_slice(format!("{edge}: 3 0 0 0 0 0 0 0 4 0\n").as_bytes());
    assert_eq!(linux::interfaces(&output), [interface(&edge, 3, 4)]);
}

#[test]
fn met_002_net_dev_empty_gives_no_interfaces() {
    assert_eq!(linux::interfaces(b"").len(), 0);
}

#[test]
fn met_002_uptime_is_the_first_number() {
    assert_eq!(linux::uptime_seconds(PROC_UPTIME), Some(350_735.47));
}

#[test]
fn met_002_uptime_rejects_nan_infinite_negative_empty_and_cut_text() {
    for bad in [
        &b"NaN 1.0\n"[..],
        b"inf 1\n",
        b"-5.0 1\n",
        b"abc\n",
        b"",
        b"\n",
        b"350735",
    ] {
        assert_eq!(linux::uptime_seconds(bad), None, "{bad:?}");
    }
}

#[test]
fn met_002_loadavg_gives_the_three_averages() {
    assert_eq!(
        linux::load_average(PROC_LOADAVG),
        Some(Load {
            one: 0.20,
            five: 0.18,
            fifteen: 0.12
        })
    );
}

#[test]
fn met_002_loadavg_with_a_missing_negative_or_nan_average_gives_none() {
    for bad in [
        &b"0.20 0.18\n"[..],
        b"0.20 -0.18 0.12\n",
        b"0.20 NaN 0.12\n",
        b"",
    ] {
        assert_eq!(linux::load_average(bad), None, "{bad:?}");
    }
}
