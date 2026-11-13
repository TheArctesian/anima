//! The change tools of the `devices` set. None of them takes a credential or the access
//! settings that decide where one is sent: those stay with the operator (MCP-004).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::DeviceTool;
use crate::devices::documents::{
    DeviceCreate, DeviceDelete, DeviceGet, DeviceUpdate, DeviceWake, device_create, device_delete,
    device_get, device_update, device_wake,
};
use crate::devices::get::resolve;
use crate::devices::list::site_id;
use crate::hub::Hub;
use crate::mcp::arguments::{confirmed, json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::sites::documents::{SiteCreate, site_create};

/// A device's operating system (K1 `OperatingSystem`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OsInput {
    Linux,
    Macos,
    Other,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInput {
    /// The site's id or exact name.
    site: String,
    /// A name for the device, unique enough for people to tell it apart.
    name: String,
    /// The host name or IP address the hub reaches it at.
    host: String,
    /// What it is, such as server, nas or router (default server).
    kind: Option<String>,
    os: Option<OsInput>,
    tags: Option<Vec<String>>,
    /// The MAC address for Wake-on-LAN.
    mac_address: Option<String>,
    /// Add a ping check (default true).
    add_default_check: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SiteInput {
    /// The site's name.
    name: String,
    /// The site's network, such as 10.10.0.0/24.
    subnet: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceInput {
    /// The device's id or exact name.
    device: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    /// The device's id or exact name. Only the fields given change.
    #[serde(skip_serializing)]
    device: String,
    name: Option<String>,
    host: Option<String>,
    kind: Option<String>,
    os: Option<OsInput>,
    /// The full new list of tags.
    tags: Option<Vec<String>>,
    /// Hold the device in maintenance (no alerts).
    maintenance: Option<bool>,
    /// An empty string clears it.
    mac_address: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    /// The device's id or exact name.
    device: String,
    /// The device's exact name, as the operator confirmed it.
    confirm: String,
}

pub fn definition(tool: DeviceTool) -> Definition {
    let device = [("device", "object", "The device: id, name, status, site.")];
    match tool {
        DeviceTool::Create => Definition::new::<CreateInput>(
            "anima_device_create",
            Access::Write,
            Effect::Adds,
            "Adds a device to a site. It gets a ping check unless addDefaultCheck is false. \
             Credentials and SSH, RDP or VNC access are set by the operator in the Anima \
             client, never here. Returns the new device.",
        )
        .returns(&device),
        DeviceTool::SiteCreate => Definition::new::<SiteInput>(
            "anima_site_create",
            Access::Write,
            Effect::Adds,
            "Adds a site: a named network where devices live. Returns the site's id.",
        )
        .returns(&[("site", "object", "The site: id, name, subnet.")]),
        DeviceTool::Wake => Definition::new::<DeviceInput>(
            "anima_device_wake",
            Access::Write,
            Effect::Acts,
            "Sends a Wake-on-LAN packet to a device from its site's wake relay. Returns when \
             and from which relay. The device may need a minute; then call anima_device_get.",
        )
        .returns(&[("wake", "object", "sentAt and relayName.")]),
        DeviceTool::Update => Definition::new::<UpdateInput>(
            "anima_device_update",
            Access::Login,
            Effect::Sets,
            "Changes a device's name, host, kind, os, tags, maintenance or MAC address; only \
             the fields given change. Returns the device.",
        )
        .returns(&device),
        _ => Definition::new::<DeleteInput>(
            "anima_device_delete",
            Access::Login,
            Effect::Removes,
            "Deletes a device with its credentials, checks and history. It cannot be undone. \
             Ask the operator first; confirm must be the device's exact name.",
        )
        .returns(&[("deleted", "string", "The deleted device's id.")]),
    }
}

pub async fn create(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CreateInput = parse(arguments)?;
    let site = site_id(hub, &input.site).await?;
    let mut fields = fields_of(&input);
    fields.remove("site");
    fields.insert("siteId".to_owned(), json!(site));
    let input: device_create::CreateDeviceInput = typed(fields)?;
    let device = hub
        .run::<DeviceCreate>(device_create::Variables { input })
        .await?
        .create_device;
    Ok(Value::Object(object([("device", json(&device))])))
}

pub async fn site_create(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: SiteInput = parse(arguments)?;
    let variables = site_create::Variables {
        name: input.name,
        subnet: input.subnet,
    };
    let site = hub.run::<SiteCreate>(variables).await?.create_site;
    Ok(Value::Object(object([("site", json(&site))])))
}

pub async fn wake(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: DeviceInput = parse(arguments)?;
    let device_id = resolve(hub, &input.device).await?;
    let wake = hub
        .run::<DeviceWake>(device_wake::Variables { device_id })
        .await?
        .wake_device;
    Ok(Value::Object(object([("wake", json(&wake))])))
}

pub async fn update(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: UpdateInput = parse(arguments)?;
    let id = resolve(hub, &input.device).await?;
    let input: device_update::UpdateDeviceInput = typed(fields_of(&input))?;
    let device = hub
        .run::<DeviceUpdate>(device_update::Variables { id, input })
        .await?
        .update_device;
    Ok(Value::Object(object([("device", json(&device))])))
}

pub async fn delete(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: DeleteInput = parse(arguments)?;
    let is_id = uuid::Uuid::parse_str(&input.device).is_ok();
    if !is_id {
        confirmed(&input.confirm, &input.device, "device")?;
    }
    let id = resolve(hub, &input.device).await?;
    let name = hub
        .run::<DeviceGet>(device_get::Variables { id: id.clone() })
        .await?
        .device
        .map(|device| device.name)
        .unwrap_or_default();
    confirmed(&input.confirm, &name, "device")?;
    let deleted = hub
        .run::<DeviceDelete>(device_delete::Variables { id })
        .await?
        .delete_device;
    Ok(Value::Object(object([("deleted", json!(deleted))])))
}

/// The given fields of `input` as K1 input fields; absent ones are left out.
fn fields_of(input: &impl Serialize) -> Map<String, Value> {
    match json(input) {
        Value::Object(fields) => fields
            .into_iter()
            .filter(|(_, value)| !value.is_null())
            .collect(),
        _ => Map::new(),
    }
}

fn typed<T: serde::de::DeserializeOwned>(fields: Map<String, Value>) -> Result<T, ToolError> {
    serde_json::from_value(Value::Object(fields))
        .map_err(|error| ToolError::Refused(format!("The device fields are not valid: {error}.")))
}
