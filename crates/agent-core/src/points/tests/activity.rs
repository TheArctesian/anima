//! Drive activity between two readings (MET-023): rates of the drives `lsblk` listed only, no
//! rate without both readings or after a counter went back.

use std::time::Duration;

use proptest::prelude::*;

use super::*;
use crate::{
    points::samples,
    readings::{DriveCounters, captures::*, drives::drives, linux},
};

fn counters(name: &str, read_sectors: u64, written_sectors: u64, busy_ms: u64) -> DriveCounters {
    DriveCounters {
        name: name.to_owned(),
        read_sectors,
        written_sectors,
        busy_ms,
    }
}

/// A pop-os reading with these counters.
fn pop_os(drive_counters: Vec<DriveCounters>) -> Reading {
    Reading {
        drives: drives(LSBLK_POP_OS),
        drive_counters,
        ..Reading::default()
    }
}

#[test]
fn met_023_read_write_and_busy_come_from_the_counter_deltas_over_the_elapsed_time() {
    let before = pop_os(vec![counters("nvme1n1", 1000, 2000, 500)]);
    // 30 seconds later: 60 000 sectors read, 120 000 written, 15 seconds busy.
    let now = pop_os(vec![counters("nvme1n1", 61_000, 122_000, 15_500)]);
    let out = samples(Some(&before), &now, Duration::from_secs(30));
    assert_close(
        value(&out, Kind::DiskReadBps, "nvme1n1").unwrap(),
        60_000.0 * 512.0 / 30.0,
    );
    assert_close(
        value(&out, Kind::DiskWriteBps, "nvme1n1").unwrap(),
        120_000.0 * 512.0 / 30.0,
    );
    assert_close(value(&out, Kind::DiskBusyPercent, "nvme1n1").unwrap(), 50.0);
}

#[test]
fn met_023_only_drives_lsblk_listed_get_activity_partitions_and_volumes_do_not() {
    let before = pop_os(linux::drive_counters(PROC_DISKSTATS));
    let now = pop_os(linux::drive_counters(PROC_DISKSTATS));
    let out = samples(Some(&before), &now, Duration::from_secs(30));
    let labels: Vec<&str> = out
        .iter()
        .filter(|sample| sample.kind == Kind::DiskReadBps)
        .map(|sample| sample.label.as_str())
        .collect();
    // nvme0n1 and sdb have no counters in the capture; loop0, nvme1n1p1 and dm-0 are not drives.
    assert_eq!(labels, ["nvme1n1", "sda"]);
    assert_close(value(&out, Kind::DiskReadBps, "sda").unwrap(), 0.0);
}

#[test]
fn met_023_no_activity_without_an_earlier_reading_after_a_wrap_or_for_a_new_drive() {
    let now = pop_os(vec![
        counters("nvme1n1", 100, 100, 100),
        counters("sda", 100, 100, 100),
    ]);
    assert_eq!(count(&samples(None, &now, THIRTY), Kind::DiskReadBps), 0);
    // nvme1n1 went back (a reboot); sda was not there before.
    let before = pop_os(vec![counters("nvme1n1", 200, 100, 100)]);
    let out = samples(Some(&before), &now, THIRTY);
    assert_eq!(count(&out, Kind::DiskReadBps), 0);
    assert_eq!(count(&out, Kind::DiskBusyPercent), 0);
    assert_eq!(
        count(
            &samples(Some(&now), &now, Duration::ZERO),
            Kind::DiskWriteBps
        ),
        0
    );
}

#[test]
fn met_023_without_a_drive_list_there_is_no_activity() {
    let mut before = pop_os(vec![counters("sda", 0, 0, 0)]);
    let mut now = pop_os(vec![counters("sda", 10, 10, 10)]);
    before.drives = None;
    now.drives = None;
    assert_eq!(samples(Some(&before), &now, THIRTY), Vec::new());
}

const THIRTY: Duration = Duration::from_secs(30);

proptest! {
    #[test]
    fn met_023_any_counters_give_finite_rates_and_a_busy_share_of_at_most_100(
        was in any::<(u64, u64, u64)>(),
        now in any::<(u64, u64, u64)>(),
        millis in 0_u64..100_000,
    ) {
        let before = pop_os(vec![counters("sda", was.0, was.1, was.2)]);
        let after = pop_os(vec![counters("sda", now.0, now.1, now.2)]);
        for sample in samples(Some(&before), &after, Duration::from_millis(millis)) {
            prop_assert!(sample.value.is_finite() && sample.value >= 0.0);
            if sample.kind == Kind::DiskBusyPercent {
                prop_assert!(sample.value <= 100.0);
            }
        }
    }
}
