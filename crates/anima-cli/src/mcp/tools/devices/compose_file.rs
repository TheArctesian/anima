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
use crate::mcp::untrusted::{BODY_CAP, capped_lines};

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
         line, a column, the number of its service and the name of a key that Compose \
         defines; never a text of the file. Next: anima_compose_from_run.",
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
             column, serviceNumber (from 1), key (only a key that Compose defines).",
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

/// What `anima_compose_from_run` does, for the agent.
const FROM_RUN_DESCRIPTION: &str = "Changes one `docker run` command that you give as text into a Compose file with one \
     service. The hub reads the words of the command with its own reader: it expands \
     nothing, runs nothing, reaches no device and keeps nothing. It refuses a command \
     with `$(`, a backtick, a pipe, `;`, `&&`, `||`, a redirection or an expansion of a \
     shell (`{a,b}`, `~`, `*`). A `$NAME` stays as text for Compose to fill. An option \
     with no key in a Compose file is not an error: its name is in notConverted. The file \
     holds the values of the command that you gave, so do not give a command with a \
     secret that you must not hold. This result removes control and invisible \
     characters from each text and cuts a long text: when that would change the file, \
     composeFile is null and withheld says why, so that you never get a changed file. \
     Do not install a file from this result without a person reading it. It installs \
     nothing. Next: anima_compose_check.";

fn from_run_definition() -> Definition {
    Definition::new::<ComposeFromRunInput>(
        "anima_compose_from_run",
        Access::Read,
        Effect::Reads,
        FROM_RUN_DESCRIPTION,
    )
    .returns(&[
        (
            "composeFile",
            "string",
            "The text of the Compose file, with no line break at its end; null when the \
             command was refused or the file is withheld.",
        ),
        (
            "fileLength",
            "integer",
            "The length of the whole file in characters; null when there is no file.",
        ),
        (
            "withheld",
            "string",
            "Null, or why the file is not given here: it is longer than 4,000 characters, or \
             it holds a character that this result removes. `anima containers from-run` \
             gives the whole file.",
        ),
        ("service", "string", "The name of the one service."),
        (
            "refusal",
            "string",
            "Null, or why the command was not converted: TOO_LARGE, NO_COMMAND, \
             NOT_DOCKER_RUN, COMMAND_SUBSTITUTION, PIPE, MORE_THAN_ONE_COMMAND, REDIRECTION, \
             SHELL_SYNTAX, VARIABLE_NOT_KEPT, ARITHMETIC_EXPANSION, BRACE_EXPANSION, \
             TILDE_EXPANSION, PATTERN, UNCLOSED_QUOTE, CONTROL_CHARACTER, \
             OPTION_WITHOUT_VALUE, NO_IMAGE, TOO_MANY_NETWORKS or FILE_TOO_LARGE.",
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
            "The names of the options of Docker that are not in the file, with no value; an \
             option that Docker does not have is `(an unknown option)`.",
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
    let (file, withheld) = given(found.compose_file.as_deref());
    let length = found.compose_file.as_ref().map(|file| file.chars().count());
    Ok(Value::Object(object([
        ("composeFile", json(&file)),
        ("fileLength", json(&length)),
        ("withheld", json(&withheld)),
        ("service", json(&found.service)),
        ("refusal", json(&found.refusal)),
        ("refusedAt", json(&found.refused_at)),
        ("notes", Value::Array(notes)),
        ("notConverted", json(&found.not_converted)),
    ])))
}

/// The file as this result can give it, or why it does not: the cleaning of each result
/// (`mcp/untrusted.rs`) must not change it, so that an agent never gets a file that differs
/// from the file of the hub. The line break at its end is the one thing that the cleaning takes.
fn given(file: Option<&str>) -> (Option<&str>, Option<&'static str>) {
    let Some(file) = file else {
        return (None, None);
    };
    let file = file.strip_suffix('\n').unwrap_or(file);
    if file.chars().count() > BODY_CAP {
        (None, Some("The file is longer than 4,000 characters."))
    } else if capped_lines(file, BODY_CAP) == file {
        (Some(file), None)
    } else {
        let why = "The file holds a character that this result removes (a control or an \
                   invisible character).";
        (None, Some(why))
    }
}

#[cfg(test)]
mod tests {
    use super::given;

    #[test]
    fn mcp_008_a_file_that_the_cleaning_would_change_is_withheld() {
        let file = "services:\n  web:\n    image: \"nginx\"\n";
        assert_eq!(given(Some(file)), (Some(file.trim_end()), None));
        assert_eq!(given(None), (None, None));
        // A zero-width space is printable YAML, but the cleaning removes it.
        let hidden = "services:\n  web:\n    image: \"ng\u{200b}inx\"\n";
        assert_eq!(given(Some(hidden)).0, None);
        let long = format!("services:\n{}", "x".repeat(4_000));
        assert_eq!(given(Some(&long)).0, None);
    }
}
