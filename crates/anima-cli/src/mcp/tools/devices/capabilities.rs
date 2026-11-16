//! `anima_capability_list`: what each device can run, from its agent's tool inventory (CAP-030,
//! CAP-031). The tool names, versions and targets come from the devices; the server cleans and
//! caps each string before the agent sees it (MCP-008).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::capabilities::list::fetch;
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityInput {
    /// Only devices where this tool is usable: git, rustc, cargo, cargo-nextest, sccache, bun,
    /// node, xcodebuild, docker, podman, nix, claude or codex.
    tool: Option<String>,
    /// Only devices with this Rust build target, for example aarch64-apple-darwin.
    build_target: Option<String>,
}

pub fn definition() -> Definition {
    Definition::new::<CapabilityInput>(
        "anima_capability_list",
        Access::Read,
        Effect::Reads,
        "What each device can run, from the tool inventory its Anima agent reports: the \
         architecture, the operating system and its version, each developer tool found \
         (git, rustc, cargo, bun, node, docker, claude, codex and others) with its version and \
         whether it is usable, and the installed Rust build targets. tool keeps the devices \
         where that tool is usable; buildTarget those with that target. A device without an \
         agent is not listed. Next: anima_device_get.",
    )
    .returns(&[
        (
            "devices",
            "array",
            "One entry per device: device (id, name), architecture, os, osVersion, tools \
             (name, version, usable), buildTargets, foundAt.",
        ),
        ("count", "integer", "How many devices are listed."),
    ])
}

pub async fn list(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CapabilityInput = parse(arguments)?;
    let found = fetch(hub, input.tool, input.build_target).await?;
    Ok(Value::Object(object([
        ("devices", json(&found)),
        ("count", json!(found.len())),
    ])))
}
