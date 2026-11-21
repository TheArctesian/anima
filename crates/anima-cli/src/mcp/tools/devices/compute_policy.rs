//! `anima_compute_policy_list` and `anima_compute_policy_set`: the device policy of each node
//! and whether it takes model work now (CMP-002, CAP-010 to CAP-012). Neither tool sets the
//! switch "takes work" (USE-004, OD-C8). Device names come from the hub; the server cleans and
//! caps each string (MCP-008).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::DeviceTool;
use crate::compute::documents::{SetDevicePolicy, set_device_policy};
use crate::compute::show::{fetch, find};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

const FIELDS: &str = "device (id, name), memorySharePercent, diskSharePercent, quietHours \
    (start, end, timeZone, or null), mainsOnly, idleOnly, isDefault, takesModelWork, \
    stopReason (NO_SWITCH, SWITCH_OFF, OFF_IN_HUB, QUIET_HOURS, ON_BATTERY, OPERATOR_ACTIVE, or \
    null when it takes model work).";

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    /// Only this device: its ID or its exact name.
    device: Option<String>,
}

/// Quiet hours: start and end as HH:MM, timeZone an IANA name.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuietInput {
    start: String,
    end: String,
    time_zone: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetInput {
    /// The device's ID or its exact name.
    device: String,
    /// 1 to 100 percent of the usable memory; unchanged when left out.
    memory_share_percent: Option<i64>,
    /// 1 to 100 percent of the disk for model files; unchanged when left out.
    disk_share_percent: Option<i64>,
    /// The new quiet hours; unchanged when left out.
    quiet_hours: Option<QuietInput>,
    /// True removes the quiet hours.
    #[serde(default)]
    no_quiet_hours: bool,
    /// Model work only on mains power; unchanged when left out.
    mains_only: Option<bool>,
    /// No new model work while the operator is active; unchanged when left out.
    idle_only: Option<bool>,
}

pub fn definition_of(tool: DeviceTool) -> Definition {
    if tool == DeviceTool::ComputePolicySet {
        return Definition::new::<SetInput>(
            "anima_compute_policy_set",
            Access::Write,
            Effect::Sets,
            "Changes the named values of the device policy of one device and keeps the others: \
             the memory share, the disk share for models, the quiet hours, mains power only, \
             and no new model work while the operator is active. It does not turn the switch \
             'takes work' on or off; that stays at the machine. Ask the operator first. Returns \
             the policy.",
        )
        .returns(&[("policy", "object", FIELDS)]);
    }
    Definition::new::<ListInput>(
        "anima_compute_policy_list",
        Access::Read,
        Effect::Reads,
        "The device policy of each device (or of one): the limits for model work, the quiet \
         hours, mains power only, no work while the operator is active, and whether the device \
         takes model work now with the reason when it does not. Next: anima_compute_policy_set.",
    )
    .returns(&[
        ("policies", "array", FIELDS),
        ("count", "integer", "How many devices are listed."),
    ])
}

pub async fn call(
    tool: DeviceTool,
    hub: &Hub,
    arguments: Option<JsonObject>,
) -> Result<Value, ToolError> {
    if tool == DeviceTool::ComputePolicySet {
        return set(hub, arguments).await;
    }
    let input: ListInput = parse(arguments)?;
    let mut rows = fetch(hub).await?;
    if let Some(wanted) = &input.device {
        rows = vec![find(&rows, wanted)?.clone()];
    }
    Ok(Value::Object(object([
        ("count", json!(rows.len())),
        ("policies", json(&rows)),
    ])))
}

async fn set(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: SetInput = parse(arguments)?;
    let rows = fetch(hub).await?;
    let current = find(&rows, &input.device)?;
    let kept = current.quiet_hours.as_ref().map(|hours| QuietInput {
        start: hours.start.clone(),
        end: hours.end.clone(),
        time_zone: hours.time_zone.clone(),
    });
    let quiet = if input.no_quiet_hours {
        None
    } else {
        input.quiet_hours.or(kept)
    };
    let variables = set_device_policy::Variables {
        device_id: current.device.id.clone(),
        input: set_device_policy::DevicePolicyInput {
            memory_share_percent: input
                .memory_share_percent
                .unwrap_or(current.memory_share_percent),
            disk_share_percent: input
                .disk_share_percent
                .unwrap_or(current.disk_share_percent),
            quiet_hours: quiet.map(|hours| set_device_policy::QuietHoursInput {
                start: hours.start,
                end: hours.end,
                time_zone: hours.time_zone,
            }),
            mains_only: input.mains_only.unwrap_or(current.mains_only),
            idle_only: input.idle_only.unwrap_or(current.idle_only),
        },
    };
    let policy = hub
        .run::<SetDevicePolicy>(variables)
        .await?
        .set_device_policy;
    Ok(Value::Object(object([("policy", json(&policy))])))
}
