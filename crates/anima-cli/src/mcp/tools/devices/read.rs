//! The read tools of the `devices` set.

use graphql_client::GraphQLQuery;
use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::devices::documents::{
    DeviceGet, DeviceMetrics, device_get, device_list, device_metrics,
};
use crate::devices::get::resolve;
use crate::devices::list::{Query, fetch, site_id};
use crate::hub::Hub;
use crate::mcp::arguments::{Page, json, object, parse, short};
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

/// How far back the metrics go.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RangeInput {
    #[default]
    Hour,
    Day,
    Week,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricsInput {
    /// The device's id or exact name.
    device: String,
    /// HOUR (default), DAY or WEEK.
    #[serde(default)]
    range: RangeInput,
    /// Add up to 60 points to each series.
    #[serde(default)]
    detail: bool,
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
const POINTS_SHOWN: usize = 60;

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
        .map(|device| json(&device.web_interfaces))
        .unwrap_or_default();
    Ok(Value::Object(object([
        ("device", short(json(&device), &GET_KEYS, input.detail)),
        ("webInterfaces", web),
    ])))
}

pub async fn metrics(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: MetricsInput = parse(arguments)?;
    let id = resolve(hub, &input.device).await?;
    let variables = device_metrics::Variables {
        id: id.clone(),
        range: input.range.k1(),
    };
    let device = hub
        .run::<DeviceMetrics>(variables)
        .await?
        .device
        .ok_or_else(|| ToolError::Refused(gone(&id)))?;
    let series: Vec<Value> = device
        .metrics
        .iter()
        .map(|series| summary(json(series), input.detail))
        .collect();
    Ok(Value::Object(object([
        ("device", json!({ "id": device.id, "name": device.name })),
        ("series", Value::Array(series)),
    ])))
}

/// One series with its last, lowest and highest value; `detail` keeps up to 60 points.
fn summary(mut series: Value, detail: bool) -> Value {
    let points = series
        .as_object_mut()
        .and_then(|fields| fields.remove("points"))
        .and_then(|points| match points {
            Value::Array(points) => Some(points),
            _ => None,
        })
        .unwrap_or_default();
    let values: Vec<f64> = points
        .iter()
        .filter_map(|point| point["value"].as_f64())
        .collect();
    let lowest = values.iter().copied().reduce(f64::min);
    let highest = values.iter().copied().reduce(f64::max);
    series["last"] = json!(values.last());
    series["lowest"] = json!(lowest);
    series["highest"] = json!(highest);
    series["pointCount"] = json!(points.len());
    if detail {
        let step = points.len().div_ceil(POINTS_SHOWN).max(1);
        let shown: Vec<Value> = points.into_iter().step_by(step).collect();
        series["points"] = Value::Array(shown);
    }
    series
}

fn gone(id: &str) -> String {
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

impl RangeInput {
    const fn k1(self) -> device_metrics::MetricRange {
        match self {
            Self::Hour => device_metrics::MetricRange::HOUR,
            Self::Day => device_metrics::MetricRange::DAY,
            Self::Week => device_metrics::MetricRange::WEEK,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_009_a_series_is_short_unless_detail_asks_for_points() {
        let points: Vec<Value> = (0..200).map(|index| json!({ "value": index })).collect();
        let series = json!({ "kind": "CPU_PERCENT", "points": points });
        let brief = summary(series.clone(), false);
        assert_eq!(
            (
                brief["last"].clone(),
                brief["lowest"].clone(),
                brief["highest"].clone()
            ),
            (json!(199.0), json!(0.0), json!(199.0))
        );
        assert!(brief.get("points").is_none());
        let full = summary(series, true);
        let shown = full["points"].as_array().map_or(0, Vec::len);
        assert!(shown <= POINTS_SHOWN && shown > 30, "{shown}");
    }
}
