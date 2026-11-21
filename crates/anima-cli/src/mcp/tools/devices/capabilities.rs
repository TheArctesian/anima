//! `anima_capability_list`: what each device can run, from its daemon's tool inventory and
//! hardware values (CAP-030 to CAP-032). The tool names, versions and targets come from the devices; the server cleans and
//! caps each string before the agent sees it (MCP-008). A tool that is not usable carries its
//! reason code and a fixed repair hint (CAP-024).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::capabilities::documents::capability_list::CapabilityListCapabilities;
use crate::capabilities::hint::{backend_hint, hint};
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
         whether it is usable (when not, a reason code and a repair hint), the installed Rust build \
         targets, and the hardware values: memory, usable GPU memory of Apple silicon, free \
         disk, CPU features, each backend (metal, cuda, rocm, vulkan, cpu) with a verdict, \
         power source, thermal state and operator activity. tool keeps the devices \
         where that tool is usable; buildTarget those with that target. A device without an \
         agent is not listed. Next: anima_device_get.",
    )
    .returns(&[
        (
            "devices",
            "array",
            "One entry per device: device (id, name), architecture, os, osVersion, tools \
             (name, version, usable, reason: TIMED_OUT, EXITED, NO_VERSION or FAILED when not \
             usable, hint: what to do then), buildTargets, foundAt, hardware (null until the \
             daemon sends it: systemMemoryBytes, unifiedMemoryBytes, dedicatedMemoryBytes, \
             usableMemoryBytes, usableMemorySource WIRED_LIMIT or DEFAULT_SHARE, freeDiskBytes, \
             cpuFeatures, backends (name, usable, reason: NO_DEVICE, NO_DRIVER, NO_PERMISSION, \
             NOT_SUPPORTED or FAILED, hint), powerSource MAINS or BATTERY, thermalState NOMINAL \
             or THROTTLED, operatorActive, foundAt).",
        ),
        ("count", "integer", "How many devices are listed."),
    ])
}

pub async fn list(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CapabilityInput = parse(arguments)?;
    let found = fetch(hub, input.tool, input.build_target).await?;
    let devices: Vec<Value> = found.iter().map(with_hints).collect();
    Ok(Value::Object(object([
        ("devices", Value::Array(devices)),
        ("count", json!(found.len())),
    ])))
}

/// The K1 entry of `row`, each tool and backend that is not usable with its repair hint.
fn with_hints(row: &CapabilityListCapabilities) -> Value {
    let mut shown = json(row);
    let tools = row.tools.iter().map(|tool| {
        let mut tool_shown = json(tool);
        if let Some(reason) = &tool.reason {
            tool_shown["hint"] = json!(hint(&tool.name, reason));
        }
        tool_shown
    });
    shown["tools"] = Value::Array(tools.collect());
    if let Some(hardware) = &row.hardware {
        let backends = hardware.backends.iter().map(|backend| {
            let mut backend_shown = json(backend);
            if let Some(reason) = &backend.reason {
                backend_shown["hint"] = json!(backend_hint(&backend.name, reason));
            }
            backend_shown
        });
        shown["hardware"]["backends"] = Value::Array(backends.collect());
    }
    shown
}
