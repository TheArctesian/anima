//! `anima_device_metrics`: a device's metric series, short by default.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::read::gone;
use crate::devices::documents::{DeviceMetrics, device_metrics};
use crate::devices::get::resolve;
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

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

const POINTS_SHOWN: usize = 60;

pub fn definition() -> Definition {
    Definition::new::<MetricsInput>(
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
    ])
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
