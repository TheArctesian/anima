//! The macOS readers (MET-002).

use chrono::DateTime;

use super::{captures::*, *};
use crate::readings::macos;

#[test]
fn met_002_sysctl_gives_memory_page_size_load_and_boot_time() {
    let read = macos::sysctl(SYSCTL);
    assert_eq!(read.memory_bytes, Some(17_179_869_184));
    assert_eq!(read.page_size, Some(16384));
    assert_eq!(
        read.load,
        Some(Load {
            one: 1.23,
            five: 1.45,
            fifteen: 1.67
        })
    );
    assert_eq!(read.boot_unix, Some(1_730_800_000));
}

#[test]
fn met_002_sysctl_uptime_is_now_less_boot_and_none_when_boot_is_in_the_future() {
    let read = macos::sysctl(SYSCTL);
    let now = DateTime::from_timestamp(1_730_803_600, 0).unwrap();
    assert_eq!(read.uptime_seconds(now), Some(3600.0));
    let before = DateTime::from_timestamp(1_730_799_999, 0).unwrap();
    assert_eq!(read.uptime_seconds(before), None);
    assert_eq!(macos::Sysctl::default().uptime_seconds(now), None);
}

#[test]
fn met_002_sysctl_with_a_bad_line_loses_only_that_field() {
    let output = b"17179869184\n\xff\xfe\n{ 1.00 NaN 2.00 }\n{ sec = -5, usec = 1 } x\n";
    let read = macos::sysctl(output);
    assert_eq!(read.memory_bytes, Some(17_179_869_184));
    assert_eq!(read.page_size, None);
    assert_eq!(read.load, None);
    assert_eq!(read.boot_unix, Some(-5));
}

#[test]
fn met_002_sysctl_empty_or_cut_gives_empty_fields() {
    assert_eq!(macos::sysctl(b""), macos::Sysctl::default());
    let cut = macos::sysctl(b"17179869184\n16384\n{ 1.23 1.45 1.67 }\n{ sec = 17308");
    assert_eq!(cut.boot_unix, None);
    assert_eq!(cut.page_size, Some(16384));
}

#[test]
fn met_002_vm_stat_used_is_app_plus_wired_plus_compressed_pages() {
    // (150000 anonymous - 5000 purgeable + 88888 wired + 120000 compressed) * 16384.
    assert_eq!(macos::memory_used_bytes(VM_STAT, None), Some(5_798_100_992));
}

#[test]
fn met_002_vm_stat_without_anonymous_pages_counts_active_pages_instead() {
    let text = String::from_utf8_lossy(VM_STAT);
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("Anonymous"))
        .collect();
    let output = format!("{}\n", kept.join("\n")).into_bytes();
    // (234567 active + 88888 wired + 120000 compressed) * 16384.
    assert_eq!(macos::memory_used_bytes(&output, None), Some(7_265_566_720));
}

#[test]
fn met_002_vm_stat_without_a_header_page_size_uses_the_sysctl_one() {
    let output = b"Pages active: 10.\nPages wired down: 5.\nPages occupied by compressor: 1.\n";
    assert_eq!(
        macos::memory_used_bytes(output, Some(4096)),
        Some(16 * 4096)
    );
    assert_eq!(macos::memory_used_bytes(output, None), None);
}

#[test]
fn met_002_vm_stat_without_wired_or_compressed_pages_or_with_overflow_gives_none() {
    assert_eq!(
        macos::memory_used_bytes(b"page size of 4096 bytes\nPages active: 10.\n", None),
        None
    );
    assert_eq!(macos::memory_used_bytes(b"", Some(4096)), None);
    let huge = b"page size of 16384 bytes\nPages active: 18446744073709551615.\nPages wired down: 1.\nPages occupied by compressor: 1.\n";
    assert_eq!(macos::memory_used_bytes(huge, None), None);
    let wide = b"page size of 16384 bytes\nPages active: 18446744073709551615.\nPages wired down: 0.\nPages occupied by compressor: 0.\n";
    assert_eq!(macos::memory_used_bytes(wide, None), None);
}

#[test]
fn met_002_vm_stat_rejects_negative_and_non_numeric_counts() {
    let output = b"page size of 4096 bytes\nPages active: -1.\nPages wired down: 1.\nPages occupied by compressor: 1.\n";
    assert_eq!(macos::memory_used_bytes(output, None), None);
}

#[test]
fn met_002_netstat_counts_only_the_link_row_of_each_interface() {
    assert_eq!(
        macos::interfaces(NETSTAT),
        [
            interface("lo0", 50_000_000, 50_000_000),
            interface("en0", 9_876_543_210, 1_234_567_890),
            interface("utun0", 0, 900),
            interface("en5", 0, 0),
        ]
    );
}

#[test]
fn met_002_netstat_skips_rows_that_do_not_read() {
    let output = b"en0 1500 <Link#4> aa:bb 1 0 x 1 0 5 0\n\
en1 1500 <Link#5> aa:bb 1 0 7 1 0 y 0\n\
en2 1500 <Link#6> 1 0\n\
en3 1500\n\
en4 1500 <Link#8> aa:bb 1 0 18446744073709551615 1 0 3 0\n";
    assert_eq!(macos::interfaces(output), [interface("en4", u64::MAX, 3)]);
}

#[test]
fn met_002_netstat_empty_gives_no_interfaces() {
    assert_eq!(macos::interfaces(b"").len(), 0);
}

#[test]
fn met_002_top_busy_share_is_100_less_idle_of_the_last_cpu_line() {
    let Some(Cpu::Percent(busy)) = macos::cpu(TOP) else {
        panic!("no reading");
    };
    assert_close(busy, 15.78);
}

#[test]
fn met_002_top_with_only_the_since_boot_line_or_a_cut_last_line_gives_none() {
    let cut =
        b"CPU usage: 8.33% user, 12.50% sys, 79.17% idle\nCPU usage: 5.26% user, 10.52% sys, 84";
    assert_eq!(macos::cpu(cut), None);
    let one = b"CPU usage: 8.33% user, 12.50% sys, 79.17% idle\n";
    assert_eq!(macos::cpu(one), None);
    let broken =
        b"CPU usage: 8.33% user, 12.50% sys, 79.17% idle\nCPU usage: 5.26% user, 10.52% sys\n";
    assert_eq!(macos::cpu(broken), None);
}

#[test]
fn met_002_top_rejects_idle_that_is_nan_negative_over_100_or_missing() {
    let first = "CPU usage: 1% user, 50.0% idle\n";
    for last in [
        "CPU usage: 1% user, NaN% idle\n",
        "CPU usage: 1% user, -4.0% idle\n",
        "CPU usage: 1% user, 140.0% idle\n",
        "CPU usage: 1% user\n",
        "Processes: 1\n",
        "",
    ] {
        assert_eq!(
            macos::cpu(format!("{first}{last}").as_bytes()),
            None,
            "{last:?}"
        );
    }
}

#[test]
fn met_002_top_all_idle_is_zero_busy_and_all_busy_is_100() {
    let idle = b"CPU usage: 9% user, 91.0% idle\nCPU usage: 0% user, 100.0% idle\n";
    assert_eq!(macos::cpu(idle), Some(Cpu::Percent(0.0)));
    let busy = b"CPU usage: 9% user, 91.0% idle\nCPU usage: 50% user, 0.0% idle\n";
    assert_eq!(macos::cpu(busy), Some(Cpu::Percent(100.0)));
}
