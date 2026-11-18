//! `anima_container_list`: the containers of a device, read through the hub's SSH link
//! (HOST-001, HOST-013). The names, images and status texts come from the device; the server
//! cleans and caps each string before the agent sees it (MCP-008). No tool returns the log of
//! a container: a log can hold a secret (MCP-004).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::containers::list::fetch;
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
         name, image, state, status text, health, ports and Compose project. It changes \
         nothing on the device. A device with no SSH credential in Anima, or with no Docker, \
         is refused with the reason. The log of a container is not available to agents: the \
         operator reads it with `anima containers logs`. Next: anima_device_get.",
    )
    .returns(&[
        (
            "containers",
            "array",
            "One entry per container, newest first: id, name, image, state (RUNNING, EXITED, \
             RESTARTING, PAUSED, CREATED, REMOVING, DEAD or UNKNOWN), status, health (HEALTHY, \
             UNHEALTHY, STARTING or NONE), ports, createdAt, composeProject, composeService.",
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
