//! `anima_container_list`, `anima_container_storage` and `anima_container_networks`: what
//! Docker has on a device, read through the hub's SSH link (HOST-001, HOST-013, HOST-019,
//! HOST-020). The names, images and status texts come from the device; the server cleans and
//! caps each string before the agent sees it (MCP-008). No tool returns the log of a container
//! or its details: a log, a command or a label can hold a secret (MCP-004, HOST-017).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::containers::list::fetch;
use crate::containers::{networks, storage};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContainerListInput {
    /// The device's id or exact name.
    device: String,
}

pub fn definition() -> Definition {
    Definition::new::<ContainerListInput>(
        "anima_container_list",
        Access::Read,
        Effect::Reads,
        "The Docker containers of one device, running or not, read now by the hub over SSH: \
         name, image, state, status text, health, ports, Compose project and the one state \
         of that project. It changes nothing on the device. A device with no SSH credential in Anima, or with no Docker, \
         is refused with the reason. The log and the details of a container are not available \
         to agents: the operator reads them with `anima containers logs` and `anima \
         containers inspect`. Next: anima_container_storage.",
    )
    .returns(&[
        (
            "containers",
            "array",
            "One entry per container, newest first: id, name, image, state (RUNNING, EXITED, \
             RESTARTING, PAUSED, CREATED, REMOVING, DEAD or UNKNOWN), status, health (HEALTHY, \
             UNHEALTHY, STARTING or NONE), ports, createdAt, composeProject, composeService, \
             composeProjectState (RUNNING, UNHEALTHY, PARTLY_RUNNING, FAILING, PAUSED, STOPPED \
             or UNKNOWN; null with no project).",
        ),
        ("count", "integer", "How many containers are listed."),
    ])
}

pub async fn list(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ContainerListInput = parse(arguments)?;
    let found = fetch(hub, &input.device).await?;
    Ok(Value::Object(object([
        ("containers", json(&found)),
        ("count", json!(found.len())),
    ])))
}

pub fn storage_definition() -> Definition {
    Definition::new::<ContainerListInput>(
        "anima_container_storage",
        Access::Read,
        Effect::Reads,
        "The Docker images and volumes of one device, read now by the hub over SSH, with the \
         size of each, the containers that use it, and the space that a clean-up can free. \
         It changes nothing on the device and removes nothing. Docker measures each volume, \
         so the read can take up to 30 seconds. A device with no SSH credential in Anima, or \
         with no Docker, is refused with the reason. Next: anima_container_networks.",
    )
    .returns(&[
        (
            "images",
            "array",
            "One entry per image: id, repository, tag, size, sharedSize, uniqueSize, created, \
             containerCount, containers. A size is the text of Docker, for example 13.6MB.",
        ),
        (
            "volumes",
            "array",
            "One entry per volume: name, driver, size, containerCount, containers.",
        ),
        (
            "usage",
            "array",
            "One entry per type (Images, Containers, Local Volumes, Build Cache): type, \
             totalCount, activeCount, size, reclaimable.",
        ),
    ])
}

pub async fn storage(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ContainerListInput = parse(arguments)?;
    let found = storage::fetch(hub, &input.device).await?;
    Ok(Value::Object(object([
        ("images", json(&found.images)),
        ("volumes", json(&found.volumes)),
        ("usage", json(&found.usage)),
    ])))
}

pub fn networks_definition() -> Definition {
    Definition::new::<ContainerListInput>(
        "anima_container_networks",
        Access::Read,
        Effect::Reads,
        "The Docker networks of one device, read now by the hub over SSH, each with the \
         containers on it. It changes nothing on the device. A device with no SSH credential \
         in Anima, or with no Docker, is refused with the reason. Next: anima_device_get.",
    )
    .returns(&[
        (
            "networks",
            "array",
            "One entry per network: id, name, driver, scope, internal, containers.",
        ),
        ("count", "integer", "How many networks are listed."),
    ])
}

pub async fn networks(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ContainerListInput = parse(arguments)?;
    let found = networks::fetch(hub, &input.device).await?;
    Ok(Value::Object(object([
        ("networks", json(&found)),
        ("count", json!(found.len())),
    ])))
}
