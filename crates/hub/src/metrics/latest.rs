//! The card readings of a device, reduced from the latest sample of each series (D-8).

use std::collections::HashMap;

use agent_core::readings::Drive;
use chrono::{DateTime, Utc};

use super::{Gpu, MetricKind, Readings};
use crate::ids::DeviceId;

/// The newest sample of one series.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Latest {
    pub(super) device: DeviceId,
    pub(super) kind: MetricKind,
    /// The mount point, interface or drive name; empty for the kinds without one.
    pub(super) label: String,
    pub(super) at: DateTime<Utc>,
    pub(super) value: f64,
    /// The stored GPU of a GPU series; `None` for the other series.
    pub(super) gpu: Option<Gpu>,
}

/// The kinds the card shows.
pub(super) const READ_KINDS: [MetricKind; 4] = [
    MetricKind::CpuPercent,
    MetricKind::MemoryPercent,
    MetricKind::DiskPercent,
    MetricKind::VramPercent,
];

/// One `Readings` per device that has a latest sample. Its time is the newest of them, and its
/// disk is the fullest mount, named by its mount point (a tie keeps the first mount bytewise).
/// VRAM is that of the GPU with VRAM whose usage is the highest; without one it stays empty,
/// never 0 (MET-007, MET-008, MET-030). The other drives are counted by [`count_other_drives`].
pub(super) fn readings(latest: Vec<Latest>) -> HashMap<DeviceId, Readings> {
    let mut by_device: HashMap<DeviceId, Readings> = HashMap::new();
    for Latest {
        device,
        kind,
        label,
        at,
        value,
        gpu,
    } in latest
    {
        let readings = by_device
            .entry(device)
            .or_insert_with(|| Readings::empty(at));
        readings.at = readings.at.max(at);
        match kind {
            MetricKind::CpuPercent => readings.cpu_percent = Some(value),
            MetricKind::MemoryPercent => readings.memory_percent = Some(value),
            MetricKind::DiskPercent => {
                let fuller = readings.disk_percent.zip(readings.disk_mount.as_deref());
                if fuller.is_none_or(|(top, held)| {
                    value.total_cmp(&top).then(held.cmp(label.as_str())).is_gt()
                }) {
                    readings.disk_percent = Some(value);
                    readings.disk_mount = Some(label);
                }
            }
            MetricKind::VramPercent => {
                // The store also drops a series without VRAM, so it cannot set `Readings.at`; this
                // keeps the rule true of the function on its own. A tie keeps the lower index.
                if let Some(gpu) = gpu.filter(|gpu| gpu.has_vram)
                    && readings
                        .vram_percent
                        .zip(readings.vram_gpu.as_ref())
                        .is_none_or(|(top, held)| {
                            value
                                .total_cmp(&top)
                                .then(held.index.cmp(&gpu.index))
                                .is_gt()
                        })
                {
                    readings.vram_percent = Some(value);
                    readings.vram_gpu = Some(gpu);
                }
            }
            _ => {}
        }
    }
    by_device
}

/// Sets each device's `other_drives`: its `drives` other than the one that holds the fullest
/// mount, unmounted ones counted, so the card can say "+N drives" (MET-030). A device whose
/// fullest mount is on no known drive counts all its drives.
pub(super) fn count_other_drives(
    readings: &mut HashMap<DeviceId, Readings>,
    drives: &HashMap<DeviceId, Vec<Drive>>,
) {
    for (device, readings) in readings.iter_mut() {
        let Some(drives) = drives.get(device) else {
            continue;
        };
        let holds = |drive: &&Drive| {
            readings
                .disk_mount
                .as_ref()
                .is_some_and(|mount| drive.mounts.contains(mount))
        };
        let others = drives.iter().filter(|drive| !holds(drive)).count();
        readings.other_drives = i32::try_from(others).unwrap_or(i32::MAX);
    }
}
