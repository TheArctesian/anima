//! What kind of work a tool call does, from the name the program gives the tool, and the one
//! line that says what it does, from its input.

use serde_json::Value;

use super::{MAX_TEXT, TITLE_LENGTH, ToolCall, first_line, held};

/// The kind of a tool call, for the icon and the words of the thread view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolKind {
    /// A command in a shell.
    Command,
    /// A change of a file.
    Edit,
    /// A read of a file or an image.
    Read,
    /// A search of files or of their text.
    Search,
    /// A fetch or a search on the web.
    Web,
    /// A sub-agent started or told something.
    Agent,
    /// A plan or a list of steps.
    Plan,
    /// A question to the person.
    Question,
    /// A tool of an MCP server, or one this reader does not know.
    Other,
}

impl ToolKind {
    /// The kind of the tool named `name` (Claude Code and Codex names).
    #[must_use]
    pub fn of(name: &str) -> Self {
        match name {
            "Bash" | "BashOutput" | "KillShell" | "shell" | "exec" | "exec_command"
            | "local_shell" | "unified_exec" | "write_stdin" => Self::Command,
            "Edit" | "MultiEdit" | "Write" | "NotebookEdit" | "apply_patch" => Self::Edit,
            "Read" | "NotebookRead" | "view_image" => Self::Read,
            "Grep" | "Glob" | "LS" | "ToolSearch" => Self::Search,
            "WebFetch" | "WebSearch" | "web_search" => Self::Web,
            "Task" | "Agent" | "spawn_agent" | "send_message" | "followup_task" | "wait"
            | "list_agents" => Self::Agent,
            "TodoWrite" | "ExitPlanMode" | "update_plan" => Self::Plan,
            "AskUserQuestion" | "request_user_input" | "request_user_input_async" => Self::Question,
            _ => Self::Other,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Edit => "edit",
            Self::Read => "read",
            Self::Search => "search",
            Self::Web => "web",
            Self::Agent => "agent",
            Self::Plan => "plan",
            Self::Question => "question",
            Self::Other => "other",
        }
    }
}

/// The call `id` of the tool `name` with its parsed `input` and the text `raw` it came from;
/// the input is shown pretty when it parsed, else as the program wrote it.
#[must_use]
pub fn call(id: &str, name: &str, input: &Value, raw: &str) -> ToolCall {
    let shown = if input.is_null() {
        raw.to_owned()
    } else {
        serde_json::to_string_pretty(input).unwrap_or_default()
    };
    ToolCall {
        call: id.to_owned(),
        name: name.to_owned(),
        kind: ToolKind::of(name),
        summary: summary(input, raw),
        patch: patch(name, input, raw),
        input: held(&shown, MAX_TEXT),
        output: None,
        failed: false,
    }
}

/// The input fields that say best what a call does, in the order they are tried.
const SUMMARY_FIELDS: [&str; 12] = [
    "command",
    "cmd",
    "file_path",
    "notebook_path",
    "path",
    "pattern",
    "url",
    "query",
    "description",
    "plan",
    "message",
    "prompt",
];

/// One line from the input of a call: its first useful field, or its first line.
#[must_use]
pub fn summary(input: &Value, raw: &str) -> String {
    let field = SUMMARY_FIELDS
        .iter()
        .find_map(|name| match input.get(name)? {
            Value::String(text) => Some(text.clone()),
            Value::Array(parts) => Some(
                parts
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => None,
        });
    let text = field.unwrap_or_else(|| patch_file(raw).unwrap_or_else(|| raw.to_owned()));
    first_line(&text, TITLE_LENGTH * 2)
}

/// The first file a patch of Codex names (`*** Update File: <path>` and its kin).
fn patch_file(raw: &str) -> Option<String> {
    raw.lines().find_map(|line| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|head| line.strip_prefix(head))
            .map(str::to_owned)
    })
}

/// The change an edit makes, as lines that start with `-`, `+` or a space; `None` for a call
/// that is not an edit or whose input this reader does not know.
#[must_use]
pub fn patch(name: &str, input: &Value, raw: &str) -> Option<String> {
    let text = |field: &str| input.get(field).and_then(Value::as_str);
    let lines = |mark: char, text: &str| {
        text.lines()
            .map(|line| format!("{mark}{line}"))
            .collect::<Vec<_>>()
    };
    match name {
        "Edit" => {
            let mut out = lines('-', text("old_string")?);
            out.extend(lines('+', text("new_string")?));
            Some(out.join("\n"))
        }
        "MultiEdit" => {
            let mut out = Vec::new();
            for edit in input.get("edits")?.as_array()? {
                out.extend(lines('-', edit.get("old_string")?.as_str()?));
                out.extend(lines('+', edit.get("new_string")?.as_str()?));
            }
            Some(out.join("\n"))
        }
        "Write" => Some(lines('+', text("content")?).join("\n")),
        "apply_patch" => Some(text("input").unwrap_or(raw).to_owned()),
        _ => None,
    }
}
