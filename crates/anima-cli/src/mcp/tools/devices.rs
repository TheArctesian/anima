//! The `devices` tool set: the fleet, devices, their metrics, sites, wake, and the changes.

mod change;
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
    Create,
    SiteCreate,
    Wake,
    Update,
    Delete,
}

impl DeviceTool {
    pub const ALL: [Self; 9] = [
        Self::FleetOverview,
        Self::Search,
        Self::Get,
        Self::Metrics,
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
            Self::Create => change::create(hub, arguments).await,
            Self::SiteCreate => change::site_create(hub, arguments).await,
            Self::Wake => change::wake(hub, arguments).await,
            Self::Update => change::update(hub, arguments).await,
            Self::Delete => change::delete(hub, arguments).await,
        }
    }
}
