//! The read tools of the `devices` set.

use graphql_client::GraphQLQuery;
use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::DeviceTool;
use crate::devices::documents::{DeviceGet, device_get, device_list};
use crate::devices::get::resolve;
use crate::devices::list::{Query, fetch, site_id};
use crate::hub::Hub;
use crate::mcp::arguments::{Page, json, object, parse, short};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::web_interfaces::documents::{WebList, web_list};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/mcp/tools/devices/overview.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct FleetOverview;

/// A tool without arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Nothing {}

/// A device's status (K1 `Status`).
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusInput {
    Up,
    Down,
    Degraded,
    Maintenance,
    Unknown,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchInput {
    /// Part of the device name, in any case.
    name: Option<String>,
    /// Only devices with one of these statuses.
    #[serde(default)]
    status: Vec<StatusInput>,
    /// Only devices with all of these tags.
    #[serde(default)]
    tags: Vec<String>,
    /// Only devices at this site: its id or exact name.
    site: Option<String>,
    /// Add host, ip, kind, os and maintenance to each device.
    #[serde(default)]
    detail: bool,
    #[serde(flatten)]
    page: Page,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetInput {
    /// The device's id or exact name.
    device: String,
    /// Add the SSH, RDP and VNC settings, metrics source and dates.
    #[serde(default)]
    detail: bool,
}

pub fn definition(tool: DeviceTool) -> Definition {
    match tool {
        DeviceTool::FleetOverview => overview_definition(),
        DeviceTool::Search => Definition::new::<SearchInput>(
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
        _ => Definition::new::<GetInput>(
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
    }
}

/// `anima_fleet_overview`: the first tool an agent calls.
fn overview_definition() -> Definition {
    Definition::new::<Nothing>(
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
    ])
}

const SEARCH_KEYS: [&str; 5] = ["id", "name", "status", "site", "tags"];
const GET_KEYS: [&str; 12] = [
    "id",
    "name",
    "status",
    "site",
    "host",
    "kind",
    "os",
    "tags",
    "maintenance",
    "checks",
    "uptime",
    "accessWarnings",
];

pub async fn overview(hub: &Hub) -> Result<Value, ToolError> {
    let answer = hub.run::<FleetOverview>(fleet_overview::Variables).await?;
    let attention: Vec<Value> = answer
        .attention
        .edges
        .iter()
        .map(|edge| json(&edge.node))
        .collect();
    let counts = json!({
        "total": answer.all.total_count, "up": answer.up.total_count,
        "down": answer.down.total_count, "degraded": answer.degraded.total_count,
        "maintenance": answer.maintenance.total_count, "unknown": answer.unknown.total_count,
    });
    Ok(Value::Object(object([
        ("counts", counts),
        ("attention", Value::Array(attention)),
        ("attentionTotal", json!(answer.attention.total_count)),
        ("sites", json(&answer.sites)),
    ])))
}

pub async fn search(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: SearchInput = parse(arguments)?;
    let site_id = match &input.site {
        Some(site) => Some(site_id(hub, site).await?),
        None => None,
    };
    let query = Query {
        filter: device_list::DeviceFilter {
            status: Some(input.status.iter().map(|status| status.k1()).collect())
                .filter(|statuses: &Vec<_>| !statuses.is_empty()),
            tags: Some(input.tags).filter(|tags| !tags.is_empty()),
            name: input.name,
            site_id,
        },
        sort: device_list::DeviceSort::NAME,
        first: input.page.size(),
    };
    let page = fetch(hub, &query, input.page.after, false).await?;
    let devices: Vec<Value> = page
        .edges
        .iter()
        .map(|edge| short(json(&edge.node), &SEARCH_KEYS, input.detail))
        .collect();
    let next = page
        .page_info
        .end_cursor
        .filter(|_| page.page_info.has_next_page);
    Ok(Value::Object(object([
        ("devices", Value::Array(devices)),
        ("totalCount", json!(page.total_count)),
        ("nextCursor", json!(next)),
    ])))
}

pub async fn get(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: GetInput = parse(arguments)?;
    let id = resolve(hub, &input.device).await?;
    let device = hub
        .run::<DeviceGet>(device_get::Variables { id: id.clone() })
        .await?
        .device
        .ok_or_else(|| ToolError::Refused(gone(&id)))?;
    let web = hub
        .run::<WebList>(web_list::Variables { id })
        .await?
        .device
        .map_or(Value::Null, |device| json(&device.web_interfaces));
    Ok(Value::Object(object([
        ("device", short(json(&device), &GET_KEYS, input.detail)),
        ("webInterfaces", web),
    ])))
}

pub fn gone(id: &str) -> String {
    format!("No device has the id {id}. Call anima_devices_search to find it.")
}

impl StatusInput {
    const fn k1(self) -> device_list::Status {
        match self {
            Self::Up => device_list::Status::UP,
            Self::Down => device_list::Status::DOWN,
            Self::Degraded => device_list::Status::DEGRADED,
            Self::Maintenance => device_list::Status::MAINTENANCE,
            Self::Unknown => device_list::Status::UNKNOWN,
        }
    }
}
