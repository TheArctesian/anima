//! Readers of the `amdgpu` sysfs files (MET-016). The caller reads the files; these read what
//! they held.

use super::{GpuReading, GpuVendor, vram};
use crate::readings::{lines, number};

/// The name given to a card whose own name is unusable.
const FALLBACK_NAME: &str = "AMD GPU";

/// What the three files of one card held.
#[derive(Clone, Copy, Debug)]
pub struct CardFiles<'a> {
    /// `gpu_busy_percent`.
    pub busy_percent: &'a [u8],
    /// `mem_info_vram_used`.
    pub vram_used: &'a [u8],
    /// `mem_info_vram_total`.
    pub vram_total: &'a [u8],
}

/// `gpu_busy_percent`: a number on one line.
#[must_use]
pub fn busy_percent(content: &[u8]) -> Option<f64> {
    number(lines(content).next()?.trim())
}

/// `mem_info_vram_used` or `mem_info_vram_total`: bytes on one line.
#[must_use]
pub fn vram_bytes(content: &[u8]) -> Option<u64> {
    lines(content).next()?.trim().parse().ok()
}

/// One card's reading. VRAM is there only when both files read and the total is not zero. A
/// `name` that cannot name a series (empty, too long, control characters) becomes "AMD GPU",
/// since the card exists whatever its name.
#[must_use]
pub fn card(index: u32, name: &str, files: CardFiles<'_>) -> GpuReading {
    GpuReading {
        index,
        name: crate::readings::label(name)
            .unwrap_or(FALLBACK_NAME)
            .to_owned(),
        vendor: GpuVendor::Amd,
        busy_percent: busy_percent(files.busy_percent),
        vram: vram(vram_bytes(files.vram_used), vram_bytes(files.vram_total)),
        temperature_c: None,
        power_w: None,
        shared_memory_bytes: None,
    }
}
