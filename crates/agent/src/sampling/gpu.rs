//! GPU readings as series and as the `gpus` list of a batch (MET-006, MET-007, K8 Section 4).
//!
//! Each vendor numbers its GPUs from 0, but a batch has one index space (0 to 63), so
//! [`Placement`] gives each vendor a base, fixed at discovery so that a GPU keeps its index
//! when another is briefly unreadable.

use agent_core::points::{Kind, MAX_GPU_INDEX, SeriesKey};
use agent_core::readings::{GpuReading, GpuVendor, Vram};
use agent_core::wire::{BatchGpu, GpuVendor as WireVendor};

/// Each vendor's first index, in the order NVIDIA, AMD, Intel, Apple.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    bases: [u32; 4],
}

const fn slot(vendor: GpuVendor) -> usize {
    match vendor {
        GpuVendor::Nvidia => 0,
        GpuVendor::Amd => 1,
        GpuVendor::Intel => 2,
        GpuVendor::Apple => 3,
    }
}

impl Placement {
    /// The bases that put each vendor's GPUs after the earlier vendors' GPUs, from what a
    /// discovery found.
    #[must_use]
    pub fn from_discovery(found: &[GpuReading]) -> Self {
        let mut counts = [0_u32; 4];
        for gpu in found {
            let count = &mut counts[slot(gpu.vendor)];
            *count = (*count).max(gpu.index.saturating_add(1));
        }
        let mut bases = [0_u32; 4];
        let mut next = 0_u32;
        for (base, count) in bases.iter_mut().zip(counts) {
            *base = next;
            next = next.saturating_add(count);
        }
        Self { bases }
    }

    /// `reading` with its index in the batch's index space; `None` past index 63.
    #[must_use]
    pub fn place(&self, mut reading: GpuReading) -> Option<GpuReading> {
        let index = self.bases[slot(reading.vendor)].checked_add(reading.index)?;
        (index <= u32::from(MAX_GPU_INDEX)).then(|| {
            reading.index = index;
            reading
        })
    }
}

/// The series values of placed GPU readings. A value the GPU did not give is left out, never
/// zero (MET-007); Apple's shared memory is `gpu_memory_bytes`, never VRAM.
#[must_use]
pub fn values(gpus: &[GpuReading]) -> Vec<(SeriesKey, f64)> {
    let mut out = Vec::new();
    for gpu in gpus {
        let Ok(index) = u8::try_from(gpu.index) else {
            continue;
        };
        let mut put = |kind: Kind, value: Option<f64>| {
            let key = SeriesKey::new(kind, None, Some(index));
            out.extend(key.zip(value));
        };
        put(Kind::GpuPercent, gpu.busy_percent);
        if let Some(Vram {
            used_bytes,
            total_bytes,
        }) = gpu.vram
        {
            put(Kind::VramBytes, Some(float(used_bytes)));
            put(
                Kind::VramPercent,
                Some(float(used_bytes.min(total_bytes)) / float(total_bytes) * 100.0),
            );
        }
        put(Kind::GpuTemperatureC, gpu.temperature_c);
        put(Kind::GpuPowerW, gpu.power_w);
        put(Kind::GpuMemoryBytes, gpu.shared_memory_bytes.map(float));
    }
    out
}

/// The `gpus` member of a batch for placed readings.
#[must_use]
pub fn announce(gpus: &[GpuReading]) -> Vec<BatchGpu> {
    gpus.iter()
        .filter_map(|gpu| {
            let (vendor, has_vram) = match gpu.vendor {
                GpuVendor::Nvidia => (WireVendor::Nvidia, true),
                GpuVendor::Amd => (WireVendor::Amd, true),
                GpuVendor::Intel => (WireVendor::Intel, false),
                GpuVendor::Apple => (WireVendor::Apple, false),
            };
            Some(BatchGpu {
                index: u8::try_from(gpu.index).ok()?,
                name: gpu.name.clone(),
                vendor,
                has_vram,
            })
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "byte counts above 2^53 lose their last digits, which a chart cannot show"
)]
const fn float(count: u64) -> f64 {
    count as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(vendor: GpuVendor, index: u32) -> GpuReading {
        GpuReading {
            index,
            name: "GPU".to_owned(),
            vendor,
            busy_percent: Some(40.0),
            vram: None,
            temperature_c: None,
            power_w: None,
            shared_memory_bytes: None,
        }
    }

    #[test]
    fn met_006_gpus_of_each_vendor_get_one_index_space() {
        let found = [
            gpu(GpuVendor::Nvidia, 0),
            gpu(GpuVendor::Nvidia, 1),
            gpu(GpuVendor::Amd, 0),
            gpu(GpuVendor::Intel, 0),
        ];
        let placement = Placement::from_discovery(&found);
        let indexes: Vec<u32> = found
            .into_iter()
            .filter_map(|reading| placement.place(reading))
            .map(|reading| reading.index)
            .collect();
        assert_eq!(indexes, [0, 1, 2, 3]);
        assert!(placement.place(gpu(GpuVendor::Intel, 64)).is_none());
    }

    #[test]
    fn met_007_a_gpu_without_vram_gives_no_vram_series_and_apple_memory_is_not_vram() {
        let mut apple = gpu(GpuVendor::Apple, 0);
        apple.shared_memory_bytes = Some(4096);
        let kinds: Vec<&str> = values(&[apple.clone()])
            .iter()
            .map(|(key, _)| key.kind().as_str())
            .collect();
        assert_eq!(kinds, ["gpu_percent", "gpu_memory_bytes"]);
        let announced = announce(&[apple]);
        assert!(!announced[0].has_vram);
        assert_eq!(announced[0].vendor, WireVendor::Apple);
    }

    #[test]
    fn met_007_vram_gives_bytes_and_a_share() {
        let mut card = gpu(GpuVendor::Nvidia, 0);
        card.vram = Some(Vram {
            used_bytes: 25,
            total_bytes: 100,
        });
        let found = values(&[card]);
        let share = found
            .iter()
            .find(|(key, _)| key.kind() == Kind::VramPercent)
            .map(|(_, value)| *value);
        assert_eq!(share, Some(25.0));
    }
}
