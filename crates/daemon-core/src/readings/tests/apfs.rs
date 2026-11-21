//! The drives of a Mac from its APFS volumes (MET-027, MET-031).

use super::{captures::*, *};
use crate::points::is_operator_mount;
use crate::readings::{Drive, DriveKind, apfs, df};

const KIB: u64 = 1024;

fn drive(name: &str, size_kib: u64, mount: &str) -> Drive {
    Drive {
        name: name.to_owned(),
        kind: DriveKind::Disk,
        size_bytes: size_kib * KIB,
        mounts: vec![mount.to_owned()],
    }
}

#[test]
fn met_031_one_container_is_one_drive_of_its_whole_size_with_its_data_volume() {
    let (disks, drives) = apfs::containers(df::disks(DF_MACOS_APFS, KIB));
    assert_eq!(
        drives,
        Some(vec![
            drive("disk3", 1_948_404_040, "/System/Volumes/Data"),
            drive("disk4", 976_762_584, "/Volumes/Archive"),
        ])
    );
    let mounts: Vec<&str> = disks
        .iter()
        .map(|disk| disk.mount.as_str())
        .filter(|mount| is_operator_mount(mount))
        .collect();
    assert_eq!(
        mounts,
        ["/System/Volumes/Data", "/Volumes/Archive", "/Volumes/share"]
    );
}

#[test]
fn met_031_a_container_with_no_data_volume_keeps_its_first_operator_mount() {
    let disks = vec![
        disk("/dev/disk5s2", "/Volumes/Two", 1_000, 100),
        disk("/dev/disk5s1", "/Volumes/One", 1_000, 200),
        disk("/dev/disk5s3", "/System/Volumes/VM", 1_000, 10),
    ];
    let (disks, drives) = apfs::containers(disks);
    assert_eq!(drives.unwrap()[0].mounts, ["/Volumes/One"]);
    assert_eq!(disks.len(), 1);
}

#[test]
fn met_031_without_apfs_volumes_every_mount_stands_alone() {
    let share = vec![disk("//user@nas/share", "/Volumes/share", 1_000, 500)];
    assert_eq!(apfs::containers(share.clone()), (share, None));
    let odd = vec![
        disk("/dev/disk", "/a", 1, 0),
        disk("/dev/diskXs1", "/b", 1, 0),
        disk("/dev/disk2", "/c", 1, 0),
    ];
    assert_eq!(apfs::containers(odd.clone()).1, None);
}
