//! The capabilities of each device: today its tool inventory, as its agent last sent it
//! (CAP-020 to CAP-031). Pure: which inventories a filter keeps, and their join with the
//! devices in device-name order. CAP-001 to CAP-004 join this when the model service is built
//! (CAP-030).

use agent_core::inventory::{Inventory, Tool};
use async_graphql::{Object, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{fleet::devices::Device, ids::DeviceId};

/// A device's tool inventory as the hub stored it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub device: DeviceId,
    pub inventory: Inventory,
    /// When the hub stored the inventory: the time of the batch that brought it.
    pub found_at: DateTime<Utc>,
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
pub fn with_devices(found: Vec<Capabilities>, devices: Vec<Device>) -> Vec<DeviceCapabilities> {
    let mut joined: Vec<DeviceCapabilities> = Vec::new();
    let mut devices = devices;
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

/// `Tool`: one tool the agent found (CAP-024).
#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "Tool")]
pub struct FoundTool {
    pub name: String,
    pub version: Option<String>,
    pub usable: bool,
}

impl From<&Tool> for FoundTool {
    fn from(tool: &Tool) -> Self {
        Self {
            name: tool.name.as_str().to_owned(),
            version: tool.version.clone(),
            usable: tool.is_usable(),
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
}
