//! The `devices` tool set: the fleet, devices, their metrics, sites, wake, and the changes.

mod change;
mod read;

use rmcp::model::JsonObject;
use serde_json::Value;

use crate::hub::Hub;
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
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
            Self::FleetOverview => Definition::new::<read::Nothing>(
                "anima_fleet_overview",
                Access::Read,
                Effect::Reads,
                "Start here. Returns how many devices the Anima hub watches, how many are UP, \
                 DOWN, DEGRADED, in MAINTENANCE or UNKNOWN, the devices that are DOWN or \
                 DEGRADED (name, id, site) and the sites. Next: anima_device_get for one \
                 device, anima_events_search for what happened.",
            )
            .returns(&[
                ("counts", "object", "Devices by status, and the total."),
                (
                    "attention",
                    "array",
                    "Devices that are DOWN or DEGRADED, worst first.",
                ),
                ("sites", "array", "Every site: id and name."),
            ]),
            Self::Search => Definition::new::<read::SearchInput>(
                "anima_devices_search",
                Access::Read,
                Effect::Reads,
                "Finds devices by part of the name, status, tag or site. Returns for each \
                 device its id, name, status, site and tags (detail adds host, ip, kind, os). \
                 Use it to turn a name into an id. Next: anima_device_get.",
            )
            .returns(&[
                ("devices", "array", "The devices of this page."),
                ("totalCount", "integer", "How many devices match in all."),
                (
                    "nextCursor",
                    "string",
                    "Give as `after` for the next page; absent at the end.",
                ),
            ]),
            Self::Get => Definition::new::<read::GetInput>(
                "anima_device_get",
                Access::Read,
                Effect::Reads,
                "One device by id or exact name: status, site, host, tags, maintenance, its \
                 checks (id, name, type, status) with uptime of the last 24 hours, its web \
                 interfaces and access warnings. detail adds SSH, RDP and VNC settings (never \
                 a credential). Next: anima_check_get for a check, anima_device_metrics.",
            )
            .returns(&[
                ("device", "object", "The device."),
                (
                    "webInterfaces",
                    "array",
                    "The device's web interfaces: id, label, url.",
                ),
            ]),
            Self::Metrics => Definition::new::<read::MetricsInput>(
                "anima_device_metrics",
                Access::Read,
                Effect::Reads,
                "A device's metrics (CPU, memory, disk, network, GPU, load) over the last \
                 HOUR, DAY or WEEK. Returns each series with its last, lowest and highest \
                 value; detail adds up to 60 points per series. Empty when the device sends \
                 no metrics.",
            )
            .returns(&[
                ("device", "object", "The device: id and name."),
                ("series", "array", "One entry per metric series."),
            ]),
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
            Self::Metrics => read::metrics(hub, arguments).await,
            Self::Create => change::create(hub, arguments).await,
            Self::SiteCreate => change::site_create(hub, arguments).await,
            Self::Wake => change::wake(hub, arguments).await,
            Self::Update => change::update(hub, arguments).await,
            Self::Delete => change::delete(hub, arguments).await,
        }
    }
}
