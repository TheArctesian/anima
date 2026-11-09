//! The GPU readers: `nvidia-smi`, the `amdgpu` sysfs files, `intel_gpu_top -J` (MET-006,
//! MET-016). Apple's GPU belongs to T5 and has its own place, [`apple`].

use std::path::Path;
use std::time::Duration;

use agent_core::readings::GpuReading;
use agent_core::readings::gpu::{amd, intel, nvidia};

use super::command::FixedCommand;

pub mod apple;

const NVIDIA_SMI: FixedCommand = FixedCommand {
    program: "nvidia-smi",
    args: &[
        "--query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
        "--format=csv,noheader,nounits",
    ],
    limit: Duration::from_secs(2),
    keep_cut_output: false,
};

const INTEL_GPU_TOP: FixedCommand = FixedCommand {
    program: "intel_gpu_top",
    args: &["-J"],
    // It prints its first sample after a second and never ends (D-30: a 2 s limit).
    limit: Duration::from_millis(1500),
    keep_cut_output: true,
};

const DRM: &str = "/sys/class/drm";

/// The AMD vendor ID in `/sys/class/drm/cardN/device/vendor`.
const AMD_VENDOR: &str = "0x1002";

/// Which GPU sources answered at the last discovery; later samplings read only those, so a
/// computer without an NVIDIA GPU does not start `nvidia-smi` every 2 seconds.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per GPU vendor; a state machine would only rename them"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    pub nvidia: bool,
    pub amd: bool,
    pub intel: bool,
    pub apple: bool,
}

impl Sources {
    /// Every source: what a discovery tries.
    pub const ALL: Self = Self {
        nvidia: true,
        amd: true,
        intel: true,
        apple: true,
    };

    /// The sources that gave a reading in `found`.
    #[must_use]
    pub fn seen(found: &[GpuReading]) -> Self {
        use agent_core::readings::GpuVendor::{Amd, Apple, Intel, Nvidia};
        let has = |vendor| found.iter().any(|gpu| gpu.vendor == vendor);
        Self {
            nvidia: has(Nvidia),
            amd: has(Amd),
            intel: has(Intel),
            apple: has(Apple),
        }
    }
}

/// Reads the GPUs of `sources`, each numbered from 0 within its vendor. A source that fails
/// gives no reading.
pub async fn read(sources: Sources, macos: bool) -> Vec<GpuReading> {
    let mut found = Vec::new();
    if macos {
        if sources.apple {
            found.extend(apple::read().await);
        }
        return found;
    }
    if sources.nvidia {
        found.extend(
            output(&NVIDIA_SMI)
                .await
                .map(|bytes| nvidia::gpus(&bytes))
                .unwrap_or_default(),
        );
    }
    if sources.amd {
        found.extend(amd_cards(Path::new(DRM)).await);
    }
    if sources.intel {
        found.extend(
            output(&INTEL_GPU_TOP)
                .await
                .and_then(|bytes| intel::gpu(&bytes)),
        );
    }
    found
}

async fn output(command: &FixedCommand) -> Option<Vec<u8>> {
    match command.run().await {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            tracing::debug!(%error, "a GPU reader gave no reading");
            None
        }
    }
}

/// One reading per AMD card under `drm` that has a `gpu_busy_percent` file, in card order.
async fn amd_cards(drm: &Path) -> Vec<GpuReading> {
    let Ok(mut entries) = tokio::fs::read_dir(drm).await else {
        return Vec::new();
    };
    let mut cards = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_card = name
            .strip_prefix("card")
            .is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()));
        if is_card {
            cards.push((name, entry.path().join("device")));
        }
    }
    cards.sort();
    let mut found = Vec::new();
    for (_, device) in cards {
        let vendor = tokio::fs::read_to_string(device.join("vendor"))
            .await
            .unwrap_or_default();
        if vendor.trim() != AMD_VENDOR {
            continue;
        }
        let file = |name: &'static str| tokio::fs::read(device.join(name));
        let Ok(busy) = file("gpu_busy_percent").await else {
            continue;
        };
        let used = file("mem_info_vram_used").await.unwrap_or_default();
        let total = file("mem_info_vram_total").await.unwrap_or_default();
        let name = tokio::fs::read_to_string(device.join("product_name"))
            .await
            .unwrap_or_default();
        let index = u32::try_from(found.len()).unwrap_or(u32::MAX);
        let files = amd::CardFiles {
            busy_percent: &busy,
            vram_used: &used,
            vram_total: &total,
        };
        found.push(amd::card(index, name.trim(), files));
    }
    found
}
