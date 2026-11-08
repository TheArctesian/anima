//! Readers of GPU output (MET-016): one [`GpuReading`] per GPU, whatever its vendor.
//!
//! A value the source does not give, or gives as `[N/A]`, `[Not Supported]` or `N/A`, is `None`,
//! never zero (MET-007). Percent values are not capped here.

pub mod amd;
pub mod apple;
pub mod intel;
pub mod nvidia;

/// Who made a GPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
}

/// The memory a GPU has of its own. Only NVIDIA and AMD discrete GPUs have it (MET-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vram {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

/// What one GPU reported in one collection.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuReading {
    pub index: u32,
    pub name: String,
    pub vendor: GpuVendor,
    /// The share of time the GPU was busy, 0 to 100.
    pub busy_percent: Option<f64>,
    pub vram: Option<Vram>,
    pub temperature_c: Option<f64>,
    pub power_w: Option<f64>,
    /// Apple silicon: system memory the GPU holds. Not a VRAM figure (MET-007).
    pub shared_memory_bytes: Option<u64>,
}

/// Video memory when both figures are known and the total is not zero.
fn vram(used_bytes: Option<u64>, total_bytes: Option<u64>) -> Option<Vram> {
    let (used_bytes, total_bytes) = (used_bytes?, total_bytes?);
    (total_bytes > 0).then_some(Vram {
        used_bytes,
        total_bytes,
    })
}
