//! The `lsblk` and `/proc/diskstats` readers (MET-022, MET-023), on the pop-os tree, old
//! util-linux output and hostile text.

use super::captures::*;
use crate::readings::{Drive, DriveCounters, DriveKind, drives::drives, linux};

fn drive(name: &str, kind: DriveKind, size_bytes: u64, mounts: &[&str]) -> Drive {
    Drive {
        name: name.to_owned(),
        kind,
        size_bytes,
        mounts: mounts.iter().map(|mount| (*mount).to_owned()).collect(),
    }
}

#[test]
fn met_022_pop_os_lists_four_drives_with_root_through_crypt_and_lvm_on_its_nvme_drive() {
    let found = drives(LSBLK_POP_OS).expect("a document");
    assert_eq!(
        found,
        [
            drive("nvme0n1", DriveKind::Nvme, 1_000_204_886_016, &[]),
            // `/` sits on data-root (LVM) in cryptdata (LUKS) on nvme1n1p3; `/recovery` is a
            // system image (D-5) and the swap is not a path.
            drive("nvme1n1", DriveKind::Nvme, 1_000_204_886_016, &["/", "/boot/efi"]),
            drive("sda", DriveKind::Hdd, 24_000_277_250_048, &[]),
            drive("sdb", DriveKind::Hdd, 8_001_563_222_016, &[]),
        ]
    );
}

#[test]
fn met_022_zram_loop_ram_and_rom_devices_are_not_drives() {
    let text = br#"{"blockdevices":[
        {"name":"zram0","type":"disk","size":1024,"rota":false},
        {"name":"loop3","type":"loop","size":1024,"rota":false},
        {"name":"ram0","type":"disk","size":1024,"rota":false},
        {"name":"sr0","type":"rom","size":1024,"rota":true},
        {"name":"sdc","type":"disk","size":0,"rota":true}]}"#;
    assert_eq!(drives(text), Some(Vec::new()));
}

#[test]
fn met_022_old_util_linux_strings_and_one_mountpoint_are_read() {
    assert_eq!(
        drives(LSBLK_OLD).expect("a document"),
        [
            drive("sda", DriveKind::Hdd, 500_107_862_016, &["/"]),
            // No transport and an empty rotation: neither says, so it is a Disk.
            drive("vda", DriveKind::Disk, 21_474_836_480, &["/srv"]),
        ]
    );
}

#[test]
fn met_022_the_kind_follows_transport_then_rotation() {
    let text = br#"{"blockdevices":[
        {"name":"a","type":"disk","size":1,"rota":true,"tran":"nvme"},
        {"name":"b","type":"disk","size":1,"rota":0,"tran":"usb"},
        {"name":"c","type":"disk","size":1,"rota":1,"tran":"sata"},
        {"name":"d","type":"disk","size":1}]}"#;
    let kinds: Vec<DriveKind> = drives(text).unwrap().iter().map(|d| d.kind).collect();
    assert_eq!(
        kinds,
        [
            DriveKind::Nvme,
            DriveKind::Ssd,
            DriveKind::Hdd,
            DriveKind::Disk
        ]
    );
    for kind in kinds {
        assert_eq!(DriveKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(DriveKind::parse("NVMe"), None);
}

#[test]
fn met_002_output_that_is_not_an_lsblk_document_gives_none() {
    for text in [
        &b""[..],
        b"lsblk: unknown column: MOUNTPOINTS\n",
        b"{\"blockdevices\": 5}",
        b"{\"blockdevices\": [{\"name\": 7}]}",
        b"[]",
        &LSBLK_POP_OS[..LSBLK_POP_OS.len() / 2],
        b"\xff\xfe{}",
    ] {
        assert_eq!(drives(text), None, "{text:?}");
    }
}

#[test]
fn met_002_hostile_names_sizes_and_mounts_are_left_out() {
    let long = "x".repeat(129);
    let text = format!(
        r#"{{"blockdevices":[
        {{"name":"{long}","type":"disk","size":1}},
        {{"name":"bad\u0007","type":"disk","size":1}},
        {{"name":"huge","type":"disk","size":18446744073709551615}},
        {{"name":"neg","type":"disk","size":-1}},
        {{"name":"ok","type":"disk","size":"10","mountpoints":["relative","/{long}","/ok\n","/proc/x","/srv"]}}]}}"#
    );
    // A negative size does not fit any form, so the whole document is refused.
    assert_eq!(drives(text.as_bytes()), None);
    let text = text.replace(r#"{"name":"neg","type":"disk","size":-1},"#, "");
    assert_eq!(
        drives(text.as_bytes()).unwrap(),
        [drive("ok", DriveKind::Disk, 10, &["/srv"])]
    );
}

#[test]
fn met_002_a_deep_tree_is_walked_eight_levels_and_counts_are_capped() {
    let mut node = r#"{"name":"leaf","type":"lvm","size":1,"mountpoints":["/deep"]}"#.to_owned();
    for level in 0..12 {
        node = format!(r#"{{"name":"n{level}","type":"part","size":1,"children":[{node}]}}"#);
    }
    let text = format!(r#"{{"blockdevices":[{{"name":"sda","type":"disk","size":1,"children":[{node}]}}]}}"#);
    assert_eq!(drives(text.as_bytes()).unwrap()[0].mounts, Vec::<String>::new());

    let many: Vec<String> = (0..40)
        .map(|i| format!(r#"{{"name":"d{i:02}","type":"disk","size":1,"mountpoints":["/m{i:02}"]}}"#))
        .collect();
    let mounts: Vec<String> = (0..40).map(|i| format!("\"/m{i:02}\"")).collect();
    let text = format!(
        r#"{{"blockdevices":[{},{{"name":"a","type":"disk","size":1,"mountpoints":[{}]}}]}}"#,
        many.join(","),
        mounts.join(",")
    );
    let found = drives(text.as_bytes()).unwrap();
    assert_eq!(found.len(), 32);
    assert_eq!(found[0].name, "a");
    assert_eq!(found[0].mounts.len(), 32);
}

#[test]
fn met_002_a_repeated_drive_name_is_kept_once() {
    let text = br#"{"blockdevices":[
        {"name":"sda","type":"disk","size":1,"mountpoints":["/a"]},
        {"name":"sda","type":"disk","size":2,"mountpoints":["/b"]}]}"#;
    assert_eq!(drives(text).unwrap().len(), 1);
}

#[test]
fn met_023_diskstats_gives_sectors_and_busy_time_of_each_device() {
    let found = linux::drive_counters(PROC_DISKSTATS);
    let names: Vec<&str> = found.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["loop0", "nvme1n1", "nvme1n1p1", "sda", "dm-0"]);
    assert_eq!(
        found[1],
        DriveCounters {
            name: "nvme1n1".to_owned(),
            read_sectors: 9_000_000,
            written_sectors: 6_000_000,
            busy_ms: 50_000,
        }
    );
}

#[test]
fn met_002_short_bad_and_cut_diskstats_lines_are_skipped() {
    let text = format!(
        "8 0 sda 1 2 3 4 5 6 7 8 9\n8 1 sdb 1 2 x 4 5 6 7 8 9 10 11\n\
         8 2 sdc 1 2 3 4 5 6 7 8 0 {max}1 11\n8 3 \u{7} 1 2 3 4 5 6 7 8 9 10 11\n\
         8 4 sdd 1 2 3 4 5 6 7 8 9 10 11\n8 5 sde 1 2 3 4 5 6 7 8 9 10",
        max = u64::MAX
    );
    let names: Vec<String> = linux::drive_counters(text.as_bytes())
        .into_iter()
        .map(|one| one.name)
        .collect();
    // sda has 12 columns, sdb's read sectors are not a number, sdc's busy time overflows, the
    // fourth name is a control character, and the last line has no newline.
    assert_eq!(names, ["sdd"]);
}

mod properties {
    use proptest::prelude::*;

    use super::*;
    use crate::points::MAX_LABELS;

    /// What every drive list must be, whatever the device printed (MET-002, MET-022).
    fn assert_kept(found: &[Drive]) {
        assert!(found.len() <= MAX_LABELS);
        assert!(found.windows(2).all(|pair| pair[0].name < pair[1].name));
        for drive in found {
            assert!((1..=128).contains(&drive.name.len()));
            assert!(!drive.name.chars().any(char::is_control));
            assert!(drive.size_bytes > 0 && i64::try_from(drive.size_bytes).is_ok());
            assert!(drive.mounts.len() <= MAX_LABELS);
            assert!(drive.mounts.iter().all(|mount| mount.starts_with('/')));
        }
    }

    proptest! {
        #[test]
        fn met_002_any_bytes_give_no_panic_and_only_kept_drives(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
            if let Some(found) = drives(&bytes) {
                assert_kept(&found);
            }
            for counters in linux::drive_counters(&bytes) {
                prop_assert!(!counters.name.is_empty() && counters.name.len() <= 128);
            }
        }

        #[test]
        fn met_002_a_changed_pop_os_document_gives_no_panic(at in 0..LSBLK_POP_OS.len(), byte in any::<u8>(), cut in 0..LSBLK_POP_OS.len()) {
            let mut changed = LSBLK_POP_OS.to_vec();
            changed[at] = byte;
            if let Some(found) = drives(&changed) {
                assert_kept(&found);
            }
            prop_assert_eq!(drives(&LSBLK_POP_OS[..cut]), None);
        }
    }
}
