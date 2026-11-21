//! The `devices` tool set: the fleet, devices, their metrics and capabilities, the use of their
//! AI agent programs, their containers, the text of a Compose file, the catalogue of
//! applications, sites, wake, and the changes.

mod agent_usage;
mod capabilities;
mod catalogue_entries;
mod change;
mod compose_file;
mod containers;
mod metrics;
mod read;
/// The public names of applications (task H21): its own tool set, kept with the other
/// application tools of a device.
pub mod routes;

use rmcp::model::JsonObject;
use serde_json::Value;

use crate::hub::Hub;
use crate::mcp::definition::Definition;
use crate::mcp::outcome::ToolError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceTool {
    FleetOverview,
    Search,
    Get,
    Metrics,
    CapabilityList,
    AgentUsage,
    ContainerList,
    ContainerStorage,
    ContainerNetworks,
    ContainerUpdates,
    ComposeCheck,
    ComposeFromRun,
    CatalogueList,
    CatalogueShow,
    CatalogueInstall,
    Create,
    SiteCreate,
    Wake,
    Update,
    Delete,
}

impl DeviceTool {
    pub const ALL: [Self; 20] = [
        Self::FleetOverview,
        Self::Search,
        Self::Get,
        Self::Metrics,
        Self::CapabilityList,
        Self::AgentUsage,
        Self::ContainerList,
        Self::ContainerStorage,
        Self::ContainerNetworks,
        Self::ContainerUpdates,
        Self::ComposeCheck,
        Self::ComposeFromRun,
        Self::CatalogueList,
        Self::CatalogueShow,
        Self::CatalogueInstall,
        Self::Create,
        Self::SiteCreate,
        Self::Wake,
        Self::Update,
        Self::Delete,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Metrics => metrics::definition(),
            Self::CapabilityList => capabilities::definition(),
            Self::AgentUsage => agent_usage::definition(),
            Self::ContainerList
            | Self::ContainerStorage
            | Self::ContainerNetworks
            | Self::ContainerUpdates => containers::definition_of(self),
            Self::ComposeCheck | Self::ComposeFromRun => compose_file::definition_of(self),
            Self::CatalogueList | Self::CatalogueShow | Self::CatalogueInstall => {
                catalogue_entries::definition_of(self)
            }
            Self::FleetOverview | Self::Search | Self::Get => read::definition(self),
            other => change::definition(other),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::FleetOverview => read::overview(hub).await,
            Self::Search => read::search(hub, arguments).await,
            Self::Get => read::get(hub, arguments).await,
            Self::Metrics => metrics::metrics(hub, arguments).await,
            Self::CapabilityList => capabilities::list(hub, arguments).await,
            Self::AgentUsage => agent_usage::usage(hub, arguments).await,
            Self::ContainerList
            | Self::ContainerStorage
            | Self::ContainerNetworks
            | Self::ContainerUpdates => containers::call(self, hub, arguments).await,
            Self::ComposeCheck | Self::ComposeFromRun => {
                compose_file::call(self, hub, arguments).await
            }
            Self::CatalogueList | Self::CatalogueShow | Self::CatalogueInstall => {
                catalogue_entries::call(self, hub, arguments).await
            }
            other => change::call(other, hub, arguments).await,
        }
    }
}
