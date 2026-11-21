//! The drives of a macOS device, read from its `df -P -k` volumes (MET-027, MET-031).
//!
//! macOS has no `lsblk`. Each APFS volume is a file system of its own in `df`, and every volume
//! of one container reports the whole container as its total, so a container with the system,
//! the data, the recovery and a `/nix` volume would be four drives whose sizes add up to four
//! times the disk. The device name tells the container: `/dev/disk3s5` and `/dev/disk3s1s1` are
//! volumes of `disk3`. So each container becomes one drive with the container's size, and of
//! its volumes only one mount stays: the data volume, which holds the operator's files, or else
//! the first operator mount by path. The other volumes of the container leave the disks, so they
//! are no series and no rows of their own. A file system that is not `/dev/disk…` (a network
//! share) stays a lone mount.

use std::collections::BTreeMap;

use super::{Disk, Drive, DriveKind, label};
use crate::points::{MAX_LABELS, is_operator_mount};

const DEVICE_PREFIX: &str = "/dev/disk";
const DATA_VOLUME: &str = "/System/Volumes/Data";

/// The container of an APFS volume's device: `disk3` for `/dev/disk3s5` or `/dev/disk3s1s1`;
/// `None` for a device of another form.
fn container(filesystem: &str) -> Option<&str> {
    let number = filesystem.strip_prefix(DEVICE_PREFIX)?;
    let digits = number.bytes().take_while(u8::is_ascii_digit).count();
    let rest = number.get(digits..)?;
    if digits == 0 || !rest.starts_with('s') {
        return None;
    }
    filesystem.get("/dev/".len()..DEVICE_PREFIX.len() + digits)
}

/// The volume of a container that stands for it: the data volume, else the first operator
/// mount by path; `None` when no volume holds the operator's files.
fn chosen(volumes: &[&Disk]) -> Option<usize> {
    volumes
        .iter()
        .position(|volume| volume.mount == DATA_VOLUME)
        .or_else(|| {
            (0..volumes.len())
                .filter(|&at| is_operator_mount(&volumes[at].mount))
                .min_by(|&a, &b| volumes[a].mount.cmp(&volumes[b].mount))
        })
}

/// The disks that keep their own series and the drives of the APFS containers among `disks`,
/// as `df -P -k` on macOS gave them. The drives are by name (bytewise), at most
/// [`MAX_LABELS`]; `None` when no disk is a volume of a container, so every mount stands alone.
#[must_use]
pub fn containers(disks: Vec<Disk>) -> (Vec<Disk>, Option<Vec<Drive>>) {
    let mut grouped: BTreeMap<String, Vec<&Disk>> = BTreeMap::new();
    for disk in &disks {
        if let Some(name) = container(&disk.filesystem).and_then(label) {
            grouped.entry(name.to_owned()).or_default().push(disk);
        }
    }
    if grouped.is_empty() {
        return (disks, None);
    }
    let mut drives = Vec::new();
    let mut kept_mounts = Vec::new();
    for (name, volumes) in &grouped {
        let Some(at) = chosen(volumes) else { continue };
        let volume = volumes[at];
        if volume.total_bytes == 0 || i64::try_from(volume.total_bytes).is_err() {
            continue;
        }
        kept_mounts.push(volume.mount.clone());
        drives.push(Drive {
            name: name.clone(),
            kind: DriveKind::Disk,
            size_bytes: volume.total_bytes,
            mounts: vec![volume.mount.clone()],
        });
    }
    drives.truncate(MAX_LABELS);
    let disks = disks
        .into_iter()
        .filter(|disk| container(&disk.filesystem).is_none() || kept_mounts.contains(&disk.mount))
        .collect();
    (disks, Some(drives))
}
