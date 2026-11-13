//! The change tools of the `checks` set.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::config::{holds_secret, typed};
use super::read::current;
use crate::checks::resolve::find;
use crate::checks::write::documents::{
    CheckCreate, CheckDelete, CheckSetPaused, CheckUpdate, check_create, check_delete,
    check_set_paused, check_update,
};
use crate::devices::get::resolve;
use crate::hub::Hub;
use crate::mcp::arguments::{confirmed, json, object, parse};
use crate::mcp::outcome::ToolError;

/// How the check runs; absent fields keep the hub's defaults or the current values.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timing {
    /// Seconds between runs (default 60).
    interval_seconds: Option<i64>,
    /// Failed runs in a row before the check is DOWN (default 3).
    retries: Option<i64>,
    /// Seconds between retries (default 20).
    retry_interval_seconds: Option<i64>,
    /// Response time in milliseconds above which the check is DEGRADED (default 1000).
    degraded_threshold_ms: Option<i64>,
    /// Count a failure as UP and a success as DOWN.
    invert: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInput {
    /// The device's id or exact name.
    device: String,
    /// The check's name, unique on the device.
    name: String,
    /// K1 `CheckConfigInput`: one key for the kind, such as `{"tcp": {"port": 22}}`.
    config: Value,
    /// A group check on the same device that holds this one: its id or exact name.
    parent: Option<String>,
    #[serde(flatten)]
    timing: Timing,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    /// The check's id, or its exact name together with `device`.
    check: String,
    /// The device's id or exact name; needed when `check` is a name.
    device: Option<String>,
    /// A new name.
    name: Option<String>,
    /// A new config (replaces the whole config), as for `anima_check_create`.
    config: Option<Value>,
    #[serde(flatten)]
    timing: Timing,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PausedInput {
    /// The check's id, or its exact name together with `device`.
    check: String,
    /// The device's id or exact name; needed when `check` is a name.
    device: Option<String>,
    /// true pauses the check, false resumes it.
    paused: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    /// The check's id, or its exact name together with `device`.
    check: String,
    /// The device's id or exact name; needed when `check` is a name.
    device: Option<String>,
    /// The check's exact name, as the operator confirmed it.
    confirm: String,
}

pub async fn create(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CreateInput = parse(arguments)?;
    let config: check_create::CheckConfigInput = typed(&input.config)?;
    let device_id = resolve(hub, &input.device).await?;
    let parent_id = match &input.parent {
        Some(parent) => Some(find(hub, parent, Some(&input.device)).await?),
        None => None,
    };
    let timing = input.timing;
    let input = check_create::CreateCheckInput {
        device_id,
        parent_id,
        name: input.name,
        config,
        interval_seconds: timing.interval_seconds,
        retries: timing.retries,
        retry_interval_seconds: timing.retry_interval_seconds,
        degraded_threshold_ms: timing.degraded_threshold_ms,
        invert: timing.invert,
    };
    let check = hub
        .run::<CheckCreate>(check_create::Variables { input })
        .await?
        .create_check;
    Ok(Value::Object(object([("check", json(&check))])))
}

pub async fn update(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: UpdateInput = parse(arguments)?;
    let config: Option<check_update::CheckConfigInput> =
        input.config.as_ref().map(typed).transpose()?;
    let id = find(hub, &input.check, input.device.as_deref()).await?;
    if config.is_some() && holds_secret(&current(hub, &id).await?["config"]) {
        return Err(ToolError::Refused(
            "This HTTP check has authentication or headers; a config from here would drop \
             them. The operator changes it with `anima check update --from-file`."
                .to_owned(),
        ));
    }
    let timing = input.timing;
    let input = check_update::UpdateCheckInput {
        parent_id: None,
        name: input.name,
        config,
        interval_seconds: timing.interval_seconds,
        retries: timing.retries,
        retry_interval_seconds: timing.retry_interval_seconds,
        degraded_threshold_ms: timing.degraded_threshold_ms,
        invert: timing.invert,
    };
    let check = hub
        .run::<CheckUpdate>(check_update::Variables { id, input })
        .await?
        .update_check;
    Ok(Value::Object(object([("check", json(&check))])))
}

pub async fn set_paused(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: PausedInput = parse(arguments)?;
    let id = find(hub, &input.check, input.device.as_deref()).await?;
    let variables = check_set_paused::Variables {
        id,
        paused: input.paused,
    };
    let check = hub.run::<CheckSetPaused>(variables).await?.set_check_paused;
    Ok(Value::Object(object([("check", json(&check))])))
}

pub async fn delete(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: DeleteInput = parse(arguments)?;
    if uuid::Uuid::parse_str(&input.check).is_err() {
        confirmed(&input.confirm, &input.check, "check")?;
    }
    let id = find(hub, &input.check, input.device.as_deref()).await?;
    let name = current(hub, &id).await?["name"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    confirmed(&input.confirm, &name, "check")?;
    let deleted = hub
        .run::<CheckDelete>(check_delete::Variables { id })
        .await?
        .delete_check;
    Ok(Value::Object(object([("deleted", json!(deleted))])))
}
