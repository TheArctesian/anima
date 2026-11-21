//! The Linux hardware values: `/proc/meminfo`, the PCI vendors of `/sys/class/drm`, the
//! presence of the NVIDIA, AMD compute and Vulkan drivers, the video memory the GPU readers give and
//! `/sys/class/power_supply` (CAP-001, CAP-002, CAP-004). Linux gives no thermal state and no
//! operator activity here.

use std::path::Path;

use daemon_core::hardware::parse::pci_vendor;
use daemon_core::hardware::power::{Supply, from_supplies};
use daemon_core::hardware::{Facts, Hardware, KfdAccess, PowerSource, backends};
use daemon_core::inventory::Os;
use daemon_core::readings::{GpuVendor, linux};

use super::{cpu_features, free_disk_bytes};
use crate::readers::gpu::{self, Sources};

const DRM: &str = "/sys/class/drm";
const POWER_SUPPLIES: &str = "/sys/class/power_supply";
const VULKAN_FOLDERS: [&str; 2] = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"];

pub(super) async fn read() -> Option<Hardware> {
    let meminfo = tokio::fs::read("/proc/meminfo").await.ok()?;
    let memory = linux::memory(&meminfo)?.total_bytes;
    let vendors = pci_vendors().await;
    let facts = Facts {
        os: Os::Linux,
        apple_silicon: false,
        nvidia_driver: exists("/proc/driver/nvidia/version").await,
        kfd: kfd_access().await,
        vulkan_driver: vulkan_driver().await,
        vendors,
    };
    Some(Hardware {
        system_memory_bytes: memory,
        unified_memory_bytes: None,
        dedicated_memory_bytes: dedicated_memory(&facts.vendors).await,
        usable_memory: None,
        free_disk_bytes: free_disk_bytes(),
        cpu_features: cpu_features(),
        backends: backends(&facts),
        power_source: Some(PowerSource::from(from_supplies(&supplies().await))),
        thermal_state: None,
        operator_active: None,
    })
}

async fn exists(path: &str) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

/// The vendors of the GPUs in `/sys/class/drm/cardN/device/vendor`, each once.
async fn pci_vendors() -> Vec<GpuVendor> {
    let mut found = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(DRM).await else {
        return found;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_card = name
            .strip_prefix("card")
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
        if !is_card {
            continue;
        }
        let vendor = tokio::fs::read_to_string(entry.path().join("device/vendor")).await;
        if let Some(vendor) = vendor.ok().as_deref().and_then(pci_vendor)
            && !found.contains(&vendor)
        {
            found.push(vendor);
        }
    }
    found
}

/// Whether the daemon's user may read and write `/dev/kfd`, the AMD compute device.
async fn kfd_access() -> KfdAccess {
    use rustix::fs::{Access, access};
    if !exists("/dev/kfd").await {
        return KfdAccess::Missing;
    }
    if access("/dev/kfd", Access::READ_OK | Access::WRITE_OK).is_ok() {
        KfdAccess::Granted
    } else {
        KfdAccess::Denied
    }
}

/// Whether a Vulkan driver manifest (`*.json`) is in one of the fixed folders.
async fn vulkan_driver() -> bool {
    for folder in VULKAN_FOLDERS {
        let Ok(mut entries) = tokio::fs::read_dir(folder).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if Path::new(&entry.file_name())
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                return true;
            }
        }
    }
    false
}

/// The sum of the video memory of the NVIDIA and AMD GPUs; `None` when none gives a total.
async fn dedicated_memory(vendors: &[GpuVendor]) -> Option<u64> {
    let sources = Sources {
        nvidia: vendors.contains(&GpuVendor::Nvidia),
        amd: vendors.contains(&GpuVendor::Amd),
        intel: false,
        apple: false,
    };
    if !(sources.nvidia || sources.amd) {
        return None;
    }
    let totals: Vec<u64> = gpu::read(sources, false)
        .await
        .iter()
        .filter_map(|found| found.vram.map(|vram| vram.total_bytes))
        .collect();
    (!totals.is_empty()).then(|| totals.iter().sum())
}

/// The entries of `/sys/class/power_supply`, as the desktop's stay-awake control reads them.
async fn supplies() -> Vec<Supply> {
    let mut found = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(POWER_SUPPLIES).await else {
        return found;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let field = |name: &str| {
            let path = path.join(name);
            async move {
                tokio::fs::read_to_string(path)
                    .await
                    .map_or_default(|text| text.trim().to_owned())
            }
        };
        found.push(Supply {
            kind: field("type").await,
            scope: field("scope").await,
            status: field("status").await,
            capacity: field("capacity").await.parse().ok().filter(|p| *p <= 100),
        });
    }
    found
}
