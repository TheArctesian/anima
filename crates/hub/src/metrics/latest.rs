//! The card readings of a device, reduced from the latest sample of each series (D-8).

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::{MetricKind, Readings};
use crate::ids::DeviceId;

/// The newest sample of one series.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Latest {
    pub(super) device: DeviceId,
    pub(super) kind: MetricKind,
    pub(super) at: DateTime<Utc>,
    pub(super) value: f64,
}

/// The kinds the card shows.
pub(super) const READ_KINDS: [MetricKind; 3] = [
    MetricKind::CpuPercent,
    MetricKind::MemoryPercent,
    MetricKind::DiskPercent,
];

/// One `Readings` per device that has a latest sample. Its time is the newest of them, and its
/// disk is the fullest mount. VRAM stays empty: nothing collects it yet (D-1).
pub(super) fn readings(latest: Vec<Latest>) -> HashMap<DeviceId, Readings> {
    let mut by_device: HashMap<DeviceId, Readings> = HashMap::new();
    for Latest {
        device,
        kind,
        at,
        value,
    } in latest
    {
        let readings = by_device.entry(device).or_insert_with(|| Readings {
            at,
            cpu_percent: None,
            memory_percent: None,
            disk_percent: None,
            vram_percent: None,
            vram_gpu: None,
        });
        readings.at = readings.at.max(at);
        match kind {
            MetricKind::CpuPercent => readings.cpu_percent = Some(value),
            MetricKind::MemoryPercent => readings.memory_percent = Some(value),
            MetricKind::DiskPercent => {
                readings.disk_percent = Some(readings.disk_percent.map_or(value, |d| d.max(value)));
            }
            _ => {}
        }
    }
    by_device
}
