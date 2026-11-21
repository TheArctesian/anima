//! `anima_catalogue_list`, `anima_catalogue_show` and `anima_catalogue_install`: the
//! catalogue of applications that ships with Anima (HOST-047, task H38). An agent may ask for
//! the install of an entry of the tested catalogue (R-234), never of a shell (OD-HOST-54); the
//! hub refuses it with a fixed reason until task H6 exists, and reaches no machine.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::DeviceTool;
use crate::catalogue::{install, list, show};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShowInput {
    /// The identifier of the entry, as `anima_catalogue_list` gives it.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallInput {
    /// The identifier of the entry, as `anima_catalogue_list` gives it.
    id: String,
    /// The device's id or exact name.
    device: String,
    /// The digest of the image (`sha256:...`, from `anima_catalogue_show`): a generated entry is
    /// installed only when the request names it.
    #[serde(default)]
    digest: Option<String>,
}

/// The definition of a catalogue tool.
pub fn definition_of(tool: DeviceTool) -> Definition {
    match tool {
        DeviceTool::CatalogueShow => show_definition(),
        DeviceTool::CatalogueInstall => install_definition(),
        _ => list_definition(),
    }
}

/// Runs a catalogue tool.
///
/// # Errors
/// [`ToolError`] for wrong arguments or the hub's refusal.
pub async fn call(
    tool: DeviceTool,
    hub: &Hub,
    arguments: Option<JsonObject>,
) -> Result<Value, ToolError> {
    match tool {
        DeviceTool::CatalogueShow => show_entry(hub, arguments).await,
        DeviceTool::CatalogueInstall => install_entry(hub, arguments).await,
        _ => list_entries(hub, arguments).await,
    }
}

fn list_definition() -> Definition {
    Definition::new::<ListInput>(
        "anima_catalogue_list",
        Access::Read,
        Effect::Reads,
        "The entries of the catalogue of applications that ships with Anima: each recipe that \
         passed the static test of the hub. No machine is reached. lastMachineTest null means \
         that the entry is not tested on a machine. Next: anima_catalogue_show.",
    )
    .returns(&[
        ("count", "integer", "How many entries are listed."),
        (
            "entries",
            "array",
            "One entry each: id, name, kind (APPLICATION or DATABASE), class (APPLICATION or \
             SHELL: a shell needs a password made on the machine, gets no public route and only \
             a person installs it), tier (TESTED by Anima or GENERATED from the data of a \
             publisher), version, tags, marks (NETWORK_PORT, ENGINE_SOCKET, HOST_NETWORK, \
             DEVICE), lastMachineTest (a day, or null: not tested on a machine), pinned (the day \
             of the last pin of the images), install (refusal and needsOpenToNetwork).",
        ),
    ])
}

async fn list_entries(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let ListInput {} = parse(arguments)?;
    let found = list::fetch(hub).await?;
    Ok(Value::Object(object([
        ("count", json!(found.len())),
        ("entries", json(&found)),
    ])))
}

fn show_definition() -> Definition {
    Definition::new::<ShowInput>(
        "anima_catalogue_show",
        Access::Read,
        Effect::Reads,
        "One entry of the catalogue: its fields, why the hub does not install it now, the \
         names of its environment variables (never a value) and its Compose file. No machine \
         is reached. Next: anima_catalogue_install.",
    )
    .returns(&[
        (
            "entry",
            "object",
            "The fields of anima_catalogue_list, and description, author, iconUrl, \
             documentationUrl, networkPorts (port and protocols), origin (publisher and read, \
             null for a tested entry), streams (port and protocols), optionalParts (kind, path, \
             on, needsMachineSetting), digests, install (refusal, reason, needsOpenToNetwork) \
             and environmentVariables (names only).",
        ),
        (
            "composeFile",
            "string",
            "The text of the Compose file. It holds no secret.",
        ),
    ])
}

async fn show_entry(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ShowInput = parse(arguments)?;
    let found = show::fetch(hub, &input.id).await?;
    let mut entry = json(&found);
    let compose = entry
        .as_object_mut()
        .and_then(|fields| fields.remove("composeFile"))
        .unwrap_or(Value::Null);
    Ok(Value::Object(object([
        ("entry", entry),
        ("composeFile", compose),
    ])))
}

fn install_definition() -> Definition {
    Definition::new::<InstallInput>(
        "anima_catalogue_install",
        Access::Write,
        Effect::Acts,
        "Asks the hub to install one entry of the catalogue on a device (R-234). Today the hub \
         always refuses with a fixed reason: the install needs task H6 of the hosted \
         applications plan, which is not built. An entry that needs the socket of the \
         container engine is never installed; a shell is installed only by a person in the \
         client, never by an agent; a generated entry needs digest, the digest of its image. \
         No machine is reached and nothing changes.",
    )
    .returns(&[
        (
            "refusal",
            "string",
            "NOT_LISTED, ENGINE_SOCKET, SHELL_NEEDS_PERSON, GENERATED_NEEDS_DIGEST or \
             INSTALL_NOT_BUILT; null when the hub would install the entry (never today).",
        ),
        ("reason", "string", "The fixed text of the refusal."),
        (
            "needsOpenToNetwork",
            "boolean",
            "The entry needs the setting Open to the network at the machine.",
        ),
    ])
}

async fn install_entry(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: InstallInput = parse(arguments)?;
    let answer = install::ask(hub, &input.id, &input.device, input.digest).await?;
    Ok(Value::Object(object([
        ("refusal", json(&answer.refusal)),
        ("reason", json!(answer.reason)),
        ("needsOpenToNetwork", json!(answer.needs_open_to_network)),
    ])))
}
