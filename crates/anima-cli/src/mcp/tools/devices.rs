//! The `devices` tool set: the fleet, devices, their metrics and capabilities, the use of their
//! AI agent programs, their containers, the text of a Compose file, sites, wake, and the changes.

mod agent_usage;
mod capabilities;
mod change;
mod compose_file;
mod compute_policy;
mod containers;
mod metrics;
mod read;

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
    ComputePolicyList,
    ComputePolicySet,
    AgentUsage,
    ContainerList,
    ContainerStorage,
    ContainerNetworks,
    ContainerUpdates,
    ComposeCheck,
    ComposeFromRun,
    Create,
    SiteCreate,
    Wake,
    Update,
    Delete,
}

impl DeviceTool {
    pub const ALL: [Self; 19] = [
        Self::FleetOverview,
        Self::Search,
        Self::Get,
        Self::Metrics,
        Self::CapabilityList,
        Self::ComputePolicyList,
        Self::AgentUsage,
        Self::ContainerList,
        Self::ContainerStorage,
        Self::ContainerNetworks,
        Self::ContainerUpdates,
        Self::ComposeCheck,
        Self::ComposeFromRun,
        Self::Create,
        Self::SiteCreate,
        Self::Wake,
        Self::Update,
        Self::Delete,
        Self::ComputePolicySet,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Metrics => metrics::definition(),
            Self::CapabilityList => capabilities::definition(),
            Self::ComputePolicyList | Self::ComputePolicySet => compute_policy::definition_of(self),
            Self::AgentUsage => agent_usage::definition(),
            Self::ContainerList
            | Self::ContainerStorage
            | Self::ContainerNetworks
            | Self::ContainerUpdates => containers::definition_of(self),
            Self::ComposeCheck | Self::ComposeFromRun => compose_file::definition_of(self),
            Self::FleetOverview | Self::Search | Self::Get => read::definition(self),
            other => change::definition(other),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    #[expect(
        clippy::cognitive_complexity,
        reason = "a flat dispatch with one arm for each tool"
    )]
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::FleetOverview => read::overview(hub).await,
            Self::Search => read::search(hub, arguments).await,
            Self::Get => read::get(hub, arguments).await,
            Self::Metrics => metrics::metrics(hub, arguments).await,
            Self::CapabilityList => capabilities::list(hub, arguments).await,
            Self::ComputePolicyList | Self::ComputePolicySet => {
                compute_policy::call(self, hub, arguments).await
            }
            Self::AgentUsage => agent_usage::usage(hub, arguments).await,
            Self::ContainerList
            | Self::ContainerStorage
            | Self::ContainerNetworks
            | Self::ContainerUpdates => containers::call(self, hub, arguments).await,
            Self::ComposeCheck | Self::ComposeFromRun => {
                compose_file::call(self, hub, arguments).await
            }
            Self::Create => change::create(hub, arguments).await,
            Self::SiteCreate => change::site_create(hub, arguments).await,
            Self::Wake => change::wake(hub, arguments).await,
            Self::Update => change::update(hub, arguments).await,
            Self::Delete => change::delete(hub, arguments).await,
        }
    }
}
