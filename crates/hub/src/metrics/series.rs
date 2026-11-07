//! Which disks and interfaces become series, so a device cannot grow the table without bound
//! (D-5).

use super::reading::{Disk, Interface};

/// The most disk series, and the most network series, one device has.
pub const MAX_LABELS: usize = 32;

const PSEUDO_FILESYSTEMS: [&str; 6] = ["tmpfs", "devtmpfs", "overlay", "squashfs", "devfs", "map"];
const PSEUDO_MOUNTS: [&str; 4] = ["/dev", "/run", "/sys", "/proc"];
const DATA_VOLUME: &str = "/System/Volumes/Data";

fn under(mount: &str, root: &str) -> bool {
    mount
        .strip_prefix(root)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Whether a disk is a real file system worth a series.
fn wanted_disk(disk: &Disk) -> bool {
    !PSEUDO_FILESYSTEMS.contains(&disk.filesystem.as_str())
        && !PSEUDO_MOUNTS.iter().any(|root| under(&disk.mount, root))
        && (disk.mount == DATA_VOLUME || !under(&disk.mount, "/System/Volumes"))
}

fn wanted_interface(interface: &Interface) -> bool {
    !matches!(interface.name.as_str(), "lo" | "lo0")
}

/// The disks that get series: real file systems only, ordered by mount point (bytewise), the
/// first of a repeated mount point, and at most [`MAX_LABELS`].
pub(super) fn kept_disks(disks: &[Disk]) -> Vec<&Disk> {
    kept(disks, wanted_disk, |disk| &disk.mount)
}

/// The interfaces that get series: not loopback, ordered by name (bytewise), the first of a
/// repeated name, and at most [`MAX_LABELS`].
pub(super) fn kept_interfaces(interfaces: &[Interface]) -> Vec<&Interface> {
    kept(interfaces, wanted_interface, |interface| &interface.name)
}

fn kept<T>(all: &[T], wanted: fn(&T) -> bool, label: fn(&T) -> &str) -> Vec<&T> {
    let mut chosen: Vec<&T> = all.iter().filter(|one| wanted(one)).collect();
    chosen.sort_by(|a, b| label(a).cmp(label(b)));
    chosen.dedup_by(|later, earlier| label(later) == label(earlier));
    chosen.truncate(MAX_LABELS);
    chosen
}
