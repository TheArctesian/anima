//! The `df -P` reader (MET-002).

use super::{captures::*, *};
use crate::readings::df;

#[test]
fn met_002_df_in_bytes_gives_every_listed_file_system_with_its_mount_point() {
    let disks = df::disks(DF_LINUX, 1);
    let mounts: Vec<&str> = disks.iter().map(|disk| disk.mount.as_str()).collect();
    assert_eq!(
        mounts,
        [
            "/dev",
            "/run",
            "/",
            "/boot/efi",
            "/mnt/Backup Drive",
            "/var/lib/docker/overlay2/abc/merged",
            "/dev/shm"
        ]
    );
    assert_eq!(
        disks[3],
        disk("/dev/nvme0n1p1", "/boot/efi", 536_870_912, 268_435_456)
    );
    assert_eq!(
        disks[4],
        disk(
            "/dev/sdb1",
            "/mnt/Backup Drive",
            1_000_000_000_000,
            100_000_000_000
        )
    );
}

#[test]
fn met_002_df_in_kibibyte_blocks_counts_bytes_in_1024s() {
    let disks = df::disks(DF_MACOS, 1024);
    assert_eq!(
        disks[0],
        disk("/dev/disk3s1s1", "/", 512_000_000_000, 128_000_000_000)
    );
    // The "map auto_home" line has a space in its file system name and is skipped.
    assert_eq!(disks.len(), 4);
}

#[test]
fn met_002_df_skips_the_header_bad_numbers_and_lines_cut_short() {
    let output = b"Filesystem 1-blocks Used Available Capacity Mounted on\n\
/dev/a x 5 5 50% /a\n\
/dev/b 10 y 5 50% /b\n\
/dev/c 10 5 5 50 /c\n\
/dev/d 10 5 5 50%\n\
/dev/e 10 5 5 50% /e\n\
/dev/f 10 5 5 50% /f";
    assert_eq!(df::disks(output, 1), [disk("/dev/e", "/e", 10, 5)]);
}

#[test]
fn met_002_df_skips_a_block_count_that_overflows_bytes_and_keeps_u64_max_bytes() {
    let output = format!("fs {m} 1 1 1% /big\nfs {m} 1 1 1% /max\n", m = u64::MAX);
    let disks = df::disks(output.as_bytes(), 1);
    assert_eq!(disks.len(), 2);
    assert_eq!(disks[1].total_bytes, u64::MAX);
    assert_eq!(df::disks(output.as_bytes(), 2).len(), 0);
}

#[test]
fn met_002_df_skips_mounts_that_are_not_utf8_empty_long_or_hold_control_characters() {
    let long = "x".repeat(129);
    let mut output = Vec::new();
    output.extend_from_slice(b"fs 10 5 5 50% /bad\xff\n");
    output.extend_from_slice(b"fs 10 5 5 50% /tab\there\n");
    output.extend_from_slice(format!("fs 10 5 5 50% /{long}\n").as_bytes());
    output.extend_from_slice(b"fs 10 5 5 50% /fine\n");
    assert_eq!(df::disks(&output, 1), [disk("fs", "/fine", 10, 5)]);
}

#[test]
fn met_002_df_empty_gives_no_disks() {
    assert_eq!(df::disks(b"", 1).len(), 0);
}
