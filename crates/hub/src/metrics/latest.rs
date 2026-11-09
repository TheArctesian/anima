//! The card readings of a device, reduced from the latest sample of each series (D-8).

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::{Gpu, MetricKind, Readings};
use crate::ids::DeviceId;

/// The newest sample of one series.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Latest {
    pub(super) device: DeviceId,
    pub(super) kind: MetricKind,
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
/// disk is the fullest mount. VRAM is that of the GPU with VRAM whose usage is the highest;
/// without one it stays empty, never 0 (MET-007, MET-008).
pub(super) fn readings(latest: Vec<Latest>) -> HashMap<DeviceId, Readings> {
    let mut by_device: HashMap<DeviceId, Readings> = HashMap::new();
    for Latest {
        device,
        kind,
        at,
        value,
        gpu,
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
