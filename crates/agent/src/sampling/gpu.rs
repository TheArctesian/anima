//! GPU readings as series and as the `gpus` list of a batch (MET-006, MET-007, K8 Section 4).
//!
//! Each vendor numbers its GPUs from 0, but a batch has one index space (0 to 63), so
//! [`Placement`] gives each vendor a base, fixed at discovery so that a GPU keeps its index
//! when another is briefly unreadable.

use std::ops::Add;
use std::time::Duration;

use agent_core::points::{Kind, MAX_GPU_INDEX, SeriesKey};
use agent_core::readings::{GpuReading, GpuVendor, Vram};
use agent_core::wire::{BatchGpu, GpuVendor as WireVendor};

use crate::readers::gpu::Sources;

/// The time to the next discovery when the GPUs are as they were (MET-006).
pub const DISCOVER_EVERY: Duration = Duration::from_hours(24);

/// The time to the next discovery after one that found fewer GPUs than before, or none the
/// first time: a driver may still be loading.
pub const DISCOVER_SOON: Duration = Duration::from_mins(5);

/// What one discovery found, and what follows from it.
#[derive(Clone, Debug, PartialEq)]
pub struct Discovery {
    /// The sources to read at each sampling.
    pub sources: Sources,
    pub placement: Placement,
    /// The `gpus` member of the next batch: the whole set, `[]` when there is none (R-098 b).
    pub announce: Vec<BatchGpu>,
    count: usize,
}

impl Discovery {
    /// The discovery of `found`, each vendor numbering its GPUs from 0.
    #[must_use]
    pub fn new(found: Vec<GpuReading>) -> Self {
        let sources = Sources::seen(&found);
        let placement = Placement::from_discovery(&found);
        let placed = placement.place_all(found);
        Self {
            sources,
            placement,
            announce: announce(&placed),
            count: placed.len(),
        }
    }

    /// When to discover again, from `now`: soon after a discovery that lost GPUs (or found
    /// none the first time), else a day.
    #[must_use]
    pub fn next_at<T: Add<Duration, Output = T>>(&self, now: T, previous: Option<&Self>) -> T {
        let lost = previous.map_or(self.count == 0, |earlier| self.count < earlier.count);
        now + if lost { DISCOVER_SOON } else { DISCOVER_EVERY }
    }
}

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

    /// The readings that have an index in the batch's index space (0 to 63), placed.
    #[must_use]
    pub fn place_all(&self, readings: Vec<GpuReading>) -> Vec<GpuReading> {
        readings
            .into_iter()
            .filter_map(|reading| self.place(reading))
            .collect()
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

/// The `gpus` member of a batch for placed readings: those with an index K8 allows (0 to
/// [`MAX_GPU_INDEX`]), as their samples are.
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
                index: u8::try_from(gpu.index)
                    .ok()
                    .filter(|index| *index <= MAX_GPU_INDEX)?,
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

    /// How long `found` waits before the next discovery.
    fn after(found: &Discovery, previous: Option<&Discovery>) -> Duration {
        let now = std::time::Instant::now();
        found.next_at(now, previous) - now
    }

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
    fn met_007_an_apple_gpu_without_a_memory_figure_gives_use_only() {
        let apple = gpu(GpuVendor::Apple, 0);
        assert!(apple.busy_percent.is_some());
        assert!(apple.shared_memory_bytes.is_none());
        let kinds: Vec<&str> = values(&[apple])
            .iter()
            .map(|(key, _)| key.kind().as_str())
            .collect();
        assert_eq!(kinds, ["gpu_percent"]);
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

    #[test]
    fn met_006_a_gpu_past_index_63_is_not_announced() {
        let found = [
            gpu(GpuVendor::Nvidia, 63),
            gpu(GpuVendor::Nvidia, 64),
            gpu(GpuVendor::Nvidia, 300),
        ];
        let indexes: Vec<u8> = announce(&found).iter().map(|gpu| gpu.index).collect();
        assert_eq!(indexes, [MAX_GPU_INDEX]);
    }

    #[test]
    fn met_006_a_discovery_that_finds_nothing_announces_an_empty_list() {
        let none = Discovery::new(Vec::new());
        assert_eq!(none.announce, []);
        assert_eq!(none.sources, Sources::default());
    }

    #[test]
    fn met_006_a_discovery_announces_every_gpu_and_names_the_sources_to_keep() {
        let found = vec![gpu(GpuVendor::Nvidia, 0), gpu(GpuVendor::Intel, 0)];
        let found = Discovery::new(found);
        let indexes: Vec<u8> = found.announce.iter().map(|gpu| gpu.index).collect();
        assert_eq!(indexes, [0, 1]);
        assert!(found.sources.nvidia && found.sources.intel);
        assert!(!found.sources.amd && !found.sources.apple);
    }

    #[test]
    fn met_006_discovery_comes_soon_after_losing_gpus_and_daily_once_stable() {
        let two = Discovery::new(vec![gpu(GpuVendor::Nvidia, 0), gpu(GpuVendor::Nvidia, 1)]);
        let one = Discovery::new(vec![gpu(GpuVendor::Nvidia, 0)]);
        let none = Discovery::new(Vec::new());
        assert_eq!(
            after(&two, None),
            DISCOVER_EVERY,
            "the first finding is trusted"
        );
        assert_eq!(after(&one, Some(&two)), DISCOVER_SOON, "fewer than before");
        assert_eq!(after(&one, Some(&one)), DISCOVER_EVERY, "stable again");
        assert_eq!(after(&none, None), DISCOVER_SOON, "none the first time");
        assert_eq!(after(&none, Some(&two)), DISCOVER_SOON, "all gone");
        assert_eq!(
            after(&none, Some(&none)),
            DISCOVER_EVERY,
            "still none: stable"
        );
        assert_eq!(after(&two, Some(&one)), DISCOVER_EVERY, "more than before");
    }
}
