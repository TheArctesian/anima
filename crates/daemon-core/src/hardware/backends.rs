//! The verdict of each model backend (CAP-002): usable, or not usable with a fixed reason. The
//! command and the MCP tool give a fixed repair hint for each reason; no text from the device
//! is sent.

use serde::{Deserialize, Serialize};

use crate::inventory::Os;
use crate::readings::GpuVendor;

/// A model backend. The order is the order of the list the daemon sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendName {
    Metal,
    Cuda,
    Rocm,
    Vulkan,
    Cpu,
    /// A backend a newer daemon knows; the hub drops it.
    #[serde(other)]
    Other,
}

impl BackendName {
    pub const ALL: [Self; 5] = [Self::Metal, Self::Cuda, Self::Rocm, Self::Vulkan, Self::Cpu];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
            Self::Other => "other",
        }
    }

    /// The backend of a stored name; `None` for a name this version does not know.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.as_str() == text)
    }
}

/// Why a backend is not usable: a fixed code, never text from the device (CAP-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendReason {
    /// No GPU of the backend's vendor is present.
    NoDevice,
    /// The GPU is present, but its kernel driver or its user-space driver is not.
    NoDriver,
    /// The driver is present, but the daemon's user may not open the device.
    NoPermission,
    /// The backend does not exist on this operating system or processor.
    NotSupported,
    /// A reason this side does not know (a newer daemon's).
    #[serde(other)]
    Failed,
}

impl BackendReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDevice => "no_device",
            Self::NoDriver => "no_driver",
            Self::NoPermission => "no_permission",
            Self::NotSupported => "not_supported",
            Self::Failed => "failed",
        }
    }

    /// The reason of a stored text; `Failed` for any other.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        [
            Self::NoDevice,
            Self::NoDriver,
            Self::NoPermission,
            Self::NotSupported,
        ]
        .into_iter()
        .find(|one| one.as_str() == text)
        .unwrap_or(Self::Failed)
    }
}

/// One backend: usable when `state` is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backend {
    pub name: BackendName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<BackendReason>,
}

impl Backend {
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        self.state.is_none()
    }
}

/// Whether the daemon's user may open `/dev/kfd`, the device of the AMD compute driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KfdAccess {
    Missing,
    Denied,
    Granted,
}

/// What the daemon read that the verdicts depend on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub os: Os,
    /// macOS on an `aarch64` processor.
    pub apple_silicon: bool,
    /// The vendors of the GPUs present: the PCI vendors of `/sys/class/drm` on Linux.
    pub vendors: Vec<GpuVendor>,
    /// `/proc/driver/nvidia/version` is present: the NVIDIA kernel driver is loaded.
    pub nvidia_driver: bool,
    pub kfd: KfdAccess,
    /// A Vulkan driver manifest is in `/usr/share/vulkan/icd.d` or `/etc/vulkan/icd.d`.
    pub vulkan_driver: bool,
}

/// The verdict of each backend in [`BackendName::ALL`] order.
#[must_use]
pub fn backends(facts: &Facts) -> Vec<Backend> {
    BackendName::ALL
        .into_iter()
        .map(|name| Backend {
            name,
            state: reason(name, facts),
        })
        .collect()
}

fn reason(name: BackendName, facts: &Facts) -> Option<BackendReason> {
    use BackendReason::{NoDevice, NoDriver, NoPermission, NotSupported};
    let has = |vendor| facts.vendors.contains(&vendor);
    let macos = facts.os == Os::Macos;
    match name {
        BackendName::Cpu => None,
        BackendName::Metal if !macos => Some(NotSupported),
        BackendName::Metal => {
            (!facts.apple_silicon && facts.vendors.is_empty()).then_some(NoDevice)
        }
        BackendName::Cuda | BackendName::Rocm | BackendName::Vulkan if macos => Some(NotSupported),
        BackendName::Cuda if !has(GpuVendor::Nvidia) => Some(NoDevice),
        BackendName::Cuda => (!facts.nvidia_driver).then_some(NoDriver),
        BackendName::Rocm if !has(GpuVendor::Amd) => Some(NoDevice),
        BackendName::Rocm => match facts.kfd {
            KfdAccess::Missing => Some(NoDriver),
            KfdAccess::Denied => Some(NoPermission),
            KfdAccess::Granted => None,
        },
        BackendName::Vulkan if facts.vendors.is_empty() => Some(NoDevice),
        BackendName::Vulkan => (!facts.vulkan_driver).then_some(NoDriver),
        BackendName::Other => Some(BackendReason::Failed),
    }
}
