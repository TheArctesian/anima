//! `anima_compose_check` and `anima_compose_from_run`: the check of the syntax of a Compose
//! file and the Compose file of a `docker run` command (HOST-022). The hub reads only the
//! text that the agent gives: it reaches no device, runs nothing and keeps nothing. A result
//! of the check holds no value of the file; the converter gives back only what the agent
//! gave, as a Compose file.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::DeviceTool;
use crate::containers::{check_file, from_run};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposeCheckInput {
    /// The text of the Compose file, at most 256 KiB.
    file: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposeFromRunInput {
    /// One `docker run` command as a person writes it in a shell, at most 64 KiB.
    command: String,
}

/// The definition of a Compose tool.
pub fn definition_of(tool: DeviceTool) -> Definition {
    match tool {
        DeviceTool::ComposeCheck => check_definition(),
        _ => from_run_definition(),
    }
}

/// Runs a Compose tool.
///
/// # Errors
/// [`ToolError`] for wrong arguments or the hub's refusal.
pub async fn call(
    tool: DeviceTool,
    hub: &Hub,
    arguments: Option<JsonObject>,
) -> Result<Value, ToolError> {
    match tool {
        DeviceTool::ComposeCheck => check(hub, arguments).await,
        _ => from_run(hub, arguments).await,
    }
}

fn check_definition() -> Definition {
    Definition::new::<ComposeCheckInput>(
        "anima_compose_check",
        Access::Read,
        Effect::Reads,
        "Checks the syntax of a Compose file that you give as text: is it YAML, and does it \
         have the form of a Compose file (services, an image or a build for each service, \
         known keys, the right form of the common keys). This is a check of the syntax only: \
         it does not say that a machine accepts the file, and it installs nothing. The hub \
         reaches no device, runs nothing and keeps nothing. A problem gives a fixed code, a \
         line, a column and at most the name of a service and of a key, never a value of \
         the file. Next: anima_compose_from_run.",
    )
    .returns(&[
        (
            "syntaxValid",
            "boolean",
            "True when the check found no error. A warning does not make it false.",
        ),
        (
            "problems",
            "array",
            "One entry per problem, in the order of the file, at most 100: code (errors: \
             TOO_LARGE, NOT_YAML, TOO_DEEP, TOO_MANY_NODES, MORE_THAN_ONE_DOCUMENT, \
             TOP_LEVEL_NOT_MAPPING, KEY_NOT_TEXT, DUPLICATE_KEY, NO_SERVICES, \
             SERVICES_NOT_MAPPING, SERVICES_EMPTY, SERVICE_NOT_MAPPING, NO_IMAGE_OR_BUILD, \
             WRONG_TYPE, RESTART_NOT_KNOWN; warnings: UNKNOWN_TOP_LEVEL_KEY, \
             UNKNOWN_SERVICE_KEY, VERSION_OBSOLETE), severity (ERROR or WARNING), line, \
             column, service, key.",
        ),
        ("errors", "integer", "How many problems are errors."),
        (
            "cut",
            "boolean",
            "True when the file has more problems than those given.",
        ),
        ("scope", "string", "What the check does not say."),
    ])
}

async fn check(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ComposeCheckInput = parse(arguments)?;
    let found = check_file::fetch(hub, input.file).await?;
    Ok(Value::Object(object([
        ("syntaxValid", json!(found.syntax_valid)),
        ("problems", json(&found.problems)),
        ("errors", json!(check_file::errors(&found))),
        ("cut", json!(found.cut)),
        ("scope", json!(check_file::SYNTAX_ONLY)),
    ])))
}

fn from_run_definition() -> Definition {
    Definition::new::<ComposeFromRunInput>(
        "anima_compose_from_run",
        Access::Read,
        Effect::Reads,
        "Changes one `docker run` command that you give as text into a Compose file with one \
         service. The hub reads the words of the command with its own reader: it expands \
         nothing, runs nothing, reaches no device and keeps nothing. It refuses a command \
         with `$(`, a backtick, a pipe, `;`, `&&`, `||` or a redirection. A `$NAME` stays as \
         text for Compose to fill. An option with no key in a Compose file is not an error: \
         its name is in notConverted. The file holds the values of the command that you \
         gave, so do not give a command with a secret that you must not hold. It installs \
         nothing. Next: anima_compose_check.",
    )
    .returns(&[
        (
            "composeFile",
            "string",
            "The text of the Compose file; null when the command was refused. A file of more \
             than 4,000 characters is cut, with a mark.",
        ),
        ("service", "string", "The name of the one service."),
        (
            "refusal",
            "string",
            "Null, or why the command was not converted: TOO_LARGE, NO_COMMAND, \
             NOT_DOCKER_RUN, COMMAND_SUBSTITUTION, PIPE, MORE_THAN_ONE_COMMAND, REDIRECTION, \
             SHELL_SYNTAX, VARIABLE_NOT_KEPT, UNCLOSED_QUOTE, CONTROL_CHARACTER, \
             OPTION_WITHOUT_VALUE or NO_IMAGE.",
        ),
        (
            "refusedAt",
            "integer",
            "The position in the command where the hub stopped, in characters from 1.",
        ),
        (
            "notes",
            "array",
            "One entry per note: code (DETACH_NOT_NEEDED, REMOVE_NOT_KEPT, VARIABLES_KEPT, \
             DOLLAR_WRITTEN, EXTERNAL_NETWORK, VOLUME_NAME_KEPT or \
             UNKNOWN_OPTION_READ_AS_FLAG) and sentence.",
        ),
        (
            "notConverted",
            "array",
            "The names of the options that are not in the file, with no value.",
        ),
    ])
}

async fn from_run(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ComposeFromRunInput = parse(arguments)?;
    let found = from_run::fetch(hub, input.command).await?;
    let notes: Vec<Value> = found
        .notes
        .iter()
        .map(|note| json!({ "code": json(note), "sentence": from_run::note_sentence(note) }))
        .collect();
    Ok(Value::Object(object([
        ("composeFile", json(&found.compose_file)),
        ("service", json(&found.service)),
        ("refusal", json(&found.refusal)),
        ("refusedAt", json(&found.refused_at)),
        ("notes", Value::Array(notes)),
        ("notConverted", json(&found.not_converted)),
    ])))
}
