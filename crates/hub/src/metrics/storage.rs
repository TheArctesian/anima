//! A device's storage drive by drive: each drive the hub knows with the latest use of the mounts
//! on it and its latest activity, then each mount that belongs to no known drive (MET-021,
//! MET-024, MET-026). Pure: the drives and the latest samples come in as parameters.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::collections::{BTreeMap, HashSet};

use agent_core::readings::{self, Drive};
use async_graphql::{Enum, SimpleObject};

use super::{MetricKind, latest::Latest};

/// The kinds the Storage section reads, newest sample of each series.
pub(super) const STORAGE_KINDS: [MetricKind; 5] = [
    MetricKind::DiskPercent,
    MetricKind::DiskBytes,
    MetricKind::DiskReadBps,
    MetricKind::DiskWriteBps,
    MetricKind::DiskBusyPercent,
];

/// What a drive is made of, detected, never entered (MET-022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum DriveKind {
    Nvme,
    Ssd,
    Hdd,
    Disk,
}

impl From<readings::DriveKind> for DriveKind {
    fn from(kind: readings::DriveKind) -> Self {
        match kind {
            readings::DriveKind::Nvme => Self::Nvme,
            readings::DriveKind::Ssd => Self::Ssd,
            readings::DriveKind::Hdd => Self::Hdd,
            readings::DriveKind::Disk => Self::Disk,
        }
    }
}

/// A mounted file system and its latest use; a value is `None` without a recent sample.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct Mount {
    pub path: String,
    pub used_bytes: Option<f64>,
    /// Used over the percent used: `None` while nothing is used, when it cannot be known.
    pub size_bytes: Option<f64>,
    pub percent: Option<f64>,
}

/// One row of the Storage section: a drive, or a mount that belongs to no drive the hub knows
/// (a network file system, every mount of a macOS device), which has no name, kind or activity.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
#[graphql(name = "Drive")]
pub struct StorageDrive {
    pub name: Option<String>,
    pub kind: Option<DriveKind>,
    pub size_bytes: Option<f64>,
    pub mounted: bool,
    pub mounts: Vec<Mount>,
    pub read_bps: Option<f64>,
    pub write_bps: Option<f64>,
    pub busy_percent: Option<f64>,
}

/// A device's storage, drive by drive (MET-021).
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct Storage {
    /// Mounted drives by name, then mounts on no known drive by path, then unmounted drives by
    /// name.
    pub drives: Vec<StorageDrive>,
}

/// The latest values of one label: a mount point's or a drive name's.
#[derive(Clone, Copy, Debug, Default)]
struct Values {
    percent: Option<f64>,
    used: Option<f64>,
    read: Option<f64>,
    write: Option<f64>,
    busy: Option<f64>,
}

/// The storage of a device with `drives` (by name) and the `latest` samples of its
/// [`STORAGE_KINDS`] series.
pub(super) fn storage(drives: &[Drive], latest: &[Latest]) -> Storage {
    let values = by_label(latest);
    let on_drives: HashSet<&str> = drives
        .iter()
        .flat_map(|drive| drive.mounts.iter().map(String::as_str))
        .collect();
    let (mounted, unmounted): (Vec<&Drive>, Vec<&Drive>) =
        drives.iter().partition(|drive| !drive.mounts.is_empty());
    let alone = values
        .iter()
        .filter(|(label, found)| found.percent.is_some() && !on_drives.contains(label.as_str()))
        .map(|(path, _)| alone_row(mount(path, &values)));
    let mut rows: Vec<StorageDrive> = mounted.iter().map(|drive| row(drive, &values)).collect();
    rows.extend(alone);
    rows.extend(unmounted.iter().map(|drive| row(drive, &values)));
    Storage { drives: rows }
}

/// Each label's latest values, by label (bytewise).
fn by_label(latest: &[Latest]) -> BTreeMap<String, Values> {
    let mut values: BTreeMap<String, Values> = BTreeMap::new();
    for one in latest {
        let slot = values.entry(one.label.clone()).or_default();
        let value = Some(one.value);
        match one.kind {
            MetricKind::DiskPercent => slot.percent = value,
            MetricKind::DiskBytes => slot.used = value,
            MetricKind::DiskReadBps => slot.read = value,
            MetricKind::DiskWriteBps => slot.write = value,
            MetricKind::DiskBusyPercent => slot.busy = value,
            _ => {}
        }
    }
    values
}

fn row(drive: &Drive, values: &BTreeMap<String, Values>) -> StorageDrive {
    let activity = values.get(&drive.name).copied().unwrap_or_default();
    StorageDrive {
        name: Some(drive.name.clone()),
        kind: Some(drive.kind.into()),
        size_bytes: Some(bytes(drive.size_bytes)),
        mounted: !drive.mounts.is_empty(),
        mounts: drive
            .mounts
            .iter()
            .map(|path| mount(path, values))
            .collect(),
        read_bps: activity.read,
        write_bps: activity.write,
        busy_percent: activity.busy,
    }
}

fn alone_row(mount: Mount) -> StorageDrive {
    StorageDrive {
        name: None,
        kind: None,
        size_bytes: mount.size_bytes,
        mounted: true,
        mounts: vec![mount],
        read_bps: None,
        write_bps: None,
        busy_percent: None,
    }
}

fn mount(path: &str, values: &BTreeMap<String, Values>) -> Mount {
    let found = values.get(path).copied().unwrap_or_default();
    let size_bytes = found
        .used
        .zip(found.percent)
        .filter(|(_, percent)| *percent > 0.0)
        .map(|(used, percent)| used / percent * 100.0);
    Mount {
        path: path.to_owned(),
        used_bytes: found.used,
        size_bytes,
        percent: found.percent,
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a drive above 2^53 bytes (8 PiB) loses its last digits, which no row shows"
)]
const fn bytes(count: u64) -> f64 {
    count as f64
}
