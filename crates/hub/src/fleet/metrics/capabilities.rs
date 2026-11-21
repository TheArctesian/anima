//! The capabilities of each device: its tool inventory and its hardware values, as its daemon
//! last sent them (CAP-001 to CAP-005, CAP-020 to CAP-032). Pure: which inventories a filter
//! keeps, and their join with the devices in device-name order.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, Object, SimpleObject};
use chrono::{DateTime, Utc};
use daemon_core::hardware::{
    Backend, BackendReason, Hardware, PowerSource, ThermalState, UsableSource,
};
use daemon_core::inventory::{Inventory, Reason, Tool};
use uuid::Uuid;

use crate::{fleet::devices::Device, ids::DeviceId};

/// A device's tool inventory as the hub stored it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub device: DeviceId,
    pub inventory: Inventory,
    /// When the hub stored the inventory: the time of the batch that brought it.
    pub found_at: DateTime<Utc>,
    /// The hardware values and when the hub stored them; `None` from a daemon older than them
    /// (CAP-032).
    pub hardware: Option<(Hardware, DateTime<Utc>)>,
}

/// Which devices `capabilities` lists (CAP-030). An empty filter keeps every device that has an
/// inventory; a name the hub does not know keeps none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityFilter {
    /// Only devices where this tool is usable.
    pub tool: Option<String>,
    /// Only devices with this Rust build target.
    pub build_target: Option<String>,
}

impl CapabilityFilter {
    /// Whether `inventory` passes both parts of the filter.
    #[must_use]
    pub fn admits(&self, inventory: &Inventory) -> bool {
        let has_tool = self.tool.as_deref().is_none_or(|wanted| {
            inventory
                .tools
                .iter()
                .any(|tool| tool.name.as_str() == wanted && tool.is_usable())
        });
        let has_target = self.build_target.as_deref().is_none_or(|wanted| {
            inventory
                .build_targets
                .iter()
                .any(|target| target == wanted)
        });
        has_tool && has_target
    }
}

/// `DeviceCapabilities`: one device and its capabilities.
pub struct DeviceCapabilities {
    device: Device,
    capabilities: Capabilities,
}

/// Each of `found` with its device, in device-name order (byte order, then ID). One whose
/// device is not in `devices` (deleted meanwhile) is left out.
#[must_use]
pub fn with_devices(found: Vec<Capabilities>, mut devices: Vec<Device>) -> Vec<DeviceCapabilities> {
    let mut joined: Vec<DeviceCapabilities> = Vec::new();
    for capabilities in found {
        let Some(at) = devices
            .iter()
            .position(|device| device.id == capabilities.device)
        else {
            continue;
        };
        let device = devices.swap_remove(at);
        joined.push(DeviceCapabilities {
            device,
            capabilities,
        });
    }
    joined.sort_by(|one, other| order(one).cmp(&order(other)));
    joined
}

/// The device's name, which `str` compares by bytes, then its ID.
fn order(row: &DeviceCapabilities) -> (&str, Uuid) {
    (row.device.name.as_str(), Uuid::from(row.device.id))
}

/// `Tool`: one tool the daemon found (CAP-024).
#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "Tool")]
pub struct FoundTool {
    pub name: String,
    pub version: Option<String>,
    pub usable: bool,
    /// Why the tool is not usable; `None` when it is.
    pub reason: Option<NotUsableReason>,
}

/// `NotUsableReason`: why a found tool is not usable, a fixed code (CAP-024).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum NotUsableReason {
    /// Its version probe ran past its time limit.
    TimedOut,
    /// It could not start, or exited with a failure.
    Exited,
    /// It ran but printed no version the daemon reads.
    NoVersion,
    /// A reason this hub does not know.
    Failed,
}

impl From<Reason> for NotUsableReason {
    fn from(reason: Reason) -> Self {
        match reason {
            Reason::TimedOut => Self::TimedOut,
            Reason::Exited => Self::Exited,
            Reason::NoVersion => Self::NoVersion,
            Reason::Failed => Self::Failed,
        }
    }
}

impl From<&Tool> for FoundTool {
    fn from(tool: &Tool) -> Self {
        Self {
            name: tool.name.as_str().to_owned(),
            version: tool.version().map(str::to_owned),
            usable: tool.is_usable(),
            reason: tool.reason().map(NotUsableReason::from),
        }
    }
}

#[Object]
impl DeviceCapabilities {
    async fn device(&self) -> &Device {
        &self.device
    }

    async fn architecture(&self) -> &str {
        self.capabilities.inventory.architecture.as_str()
    }

    async fn os(&self) -> &str {
        self.capabilities.inventory.os.as_str()
    }

    async fn os_version(&self) -> &str {
        &self.capabilities.inventory.os_version
    }

    async fn tools(&self) -> Vec<FoundTool> {
        let tools = &self.capabilities.inventory.tools;
        tools.iter().map(FoundTool::from).collect()
    }

    async fn build_targets(&self) -> &[String] {
        &self.capabilities.inventory.build_targets
    }

    async fn found_at(&self) -> DateTime<Utc> {
        self.capabilities.found_at
    }

    /// The hardware values; null until the daemon sends them (CAP-032, R-165).
    async fn hardware(&self) -> Option<HardwareValues> {
        let (hardware, found_at) = self.capabilities.hardware.as_ref()?;
        Some(HardwareValues::of(hardware, *found_at))
    }
}

/// `Hardware`: the hardware values a daemon last sent (CAP-001 to CAP-005). Byte counts are
/// `Float`, as the drive sizes are; null is a value the daemon could not read.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
#[graphql(name = "Hardware")]
pub struct HardwareValues {
    pub system_memory_bytes: f64,
    pub unified_memory_bytes: Option<f64>,
    pub dedicated_memory_bytes: Option<f64>,
    pub usable_memory_bytes: Option<f64>,
    pub usable_memory_source: Option<UsableMemorySource>,
    pub free_disk_bytes: Option<f64>,
    pub cpu_features: Vec<String>,
    pub backends: Vec<BackendVerdict>,
    pub power_source: Option<PowerState>,
    pub thermal_state: Option<Thermal>,
    pub operator_active: Option<bool>,
    pub found_at: DateTime<Utc>,
}

/// `Backend`: one model backend and its verdict (CAP-002).
#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "Backend")]
pub struct BackendVerdict {
    pub name: String,
    pub usable: bool,
    pub reason: Option<BackendNotUsable>,
}

/// `BackendNotUsable`: why a backend is not usable, a fixed code (CAP-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum BackendNotUsable {
    NoDevice,
    NoDriver,
    NoPermission,
    NotSupported,
    Failed,
}

/// `UsableMemorySource`: where the usable memory of Apple silicon came from (CAP-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UsableMemorySource {
    WiredLimit,
    DefaultShare,
}

/// `PowerSource` (CAP-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "PowerSource")]
pub enum PowerState {
    Mains,
    Battery,
}

/// `ThermalState` (CAP-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "ThermalState")]
pub enum Thermal {
    Nominal,
    Throttled,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "byte counts are Float in K1, as drive sizes"
)]
const fn float(bytes: u64) -> f64 {
    bytes as f64
}

impl HardwareValues {
    fn of(hardware: &Hardware, found_at: DateTime<Utc>) -> Self {
        let usable = hardware.usable_memory;
        Self {
            system_memory_bytes: float(hardware.system_memory_bytes),
            unified_memory_bytes: hardware.unified_memory_bytes.map(float),
            dedicated_memory_bytes: hardware.dedicated_memory_bytes.map(float),
            usable_memory_bytes: usable.map(|usable| float(usable.bytes)),
            usable_memory_source: usable.and_then(|usable| match usable.source {
                UsableSource::WiredLimit => Some(UsableMemorySource::WiredLimit),
                UsableSource::DefaultShare => Some(UsableMemorySource::DefaultShare),
                UsableSource::Unknown => None,
            }),
            free_disk_bytes: hardware.free_disk_bytes.map(float),
            cpu_features: hardware.cpu_features.clone(),
            backends: hardware.backends.iter().map(BackendVerdict::from).collect(),
            power_source: hardware.power_source.and_then(|source| match source {
                PowerSource::Mains => Some(PowerState::Mains),
                PowerSource::Battery => Some(PowerState::Battery),
                PowerSource::Unknown => None,
            }),
            thermal_state: hardware.thermal_state.and_then(|state| match state {
                ThermalState::Nominal => Some(Thermal::Nominal),
                ThermalState::Throttled => Some(Thermal::Throttled),
                ThermalState::Unknown => None,
            }),
            operator_active: hardware.operator_active,
            found_at,
        }
    }
}

impl From<&Backend> for BackendVerdict {
    fn from(backend: &Backend) -> Self {
        Self {
            name: backend.name.as_str().to_owned(),
            usable: backend.is_usable(),
            reason: backend.state.map(|reason| match reason {
                BackendReason::NoDevice => BackendNotUsable::NoDevice,
                BackendReason::NoDriver => BackendNotUsable::NoDriver,
                BackendReason::NoPermission => BackendNotUsable::NoPermission,
                BackendReason::NotSupported => BackendNotUsable::NotSupported,
                BackendReason::Failed => BackendNotUsable::Failed,
            }),
        }
    }
}
