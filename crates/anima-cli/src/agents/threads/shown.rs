//! One thread as a person reads it in a terminal, and as `--json` and the MCP tool give it.

use std::fmt::Write as _;

use chrono::{DateTime, Utc};
use daemon_core::agent_threads::{Entry, EntryKind, Thread};
use serde::Serialize;

use super::ThreadRow;
use super::files::SessionFile;

/// The lines of a tool's output a terminal shows; `--json` has all that was kept.
const OUTPUT_LINES: usize = 12;

/// The words of a program.
#[must_use]
pub fn program_words(program: &str) -> &'static str {
    match program {
        "claude_code" => "Claude Code",
        "codex" => "Codex",
        _ => "Other",
    }
}

/// A thread with its entries (`--json` field names).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    #[serde(flatten)]
    pub row: ThreadRow,
    pub prompts: u64,
    pub entries: Vec<ShownEntry>,
    /// Entries past the limit of a thread, not shown.
    pub dropped: u64,
}

/// One entry: `kind` is `prompt`, `reply`, `reasoning`, `tool`, `turn_end` or `compacted`.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownEntry {
    pub kind: &'static str,
    pub at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ShownTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownTool {
    pub name: String,
    pub kind: &'static str,
    pub summary: String,
    pub input: String,
    pub patch: Option<String>,
    pub output: Option<String>,
    pub failed: bool,
}

impl Shown {
    #[must_use]
    pub fn of(file: &SessionFile, thread: &Thread) -> Self {
        Self {
            row: ThreadRow::of(file, thread.clone()),
            prompts: thread.prompts,
            entries: thread.entries.iter().map(ShownEntry::of).collect(),
            dropped: thread.dropped,
        }
    }
}

impl ShownEntry {
    fn of(entry: &Entry) -> Self {
        let at = entry.at;
        match &entry.kind {
            EntryKind::Prompt { text, images } => Self {
                kind: "prompt",
                at,
                text: Some(text.clone()),
                images: Some(*images),
                ..Self::default()
            },
            EntryKind::Reply { text } => Self::text("reply", at, text),
            EntryKind::Reasoning { text } => Self::text("reasoning", at, text),
            EntryKind::Tool(call) => Self {
                kind: "tool",
                at,
                tool: Some(ShownTool {
                    name: call.name.clone(),
                    kind: call.kind.as_str(),
                    summary: call.summary.clone(),
                    input: call.input.clone(),
                    patch: call.patch.clone(),
                    output: call.output.clone(),
                    failed: call.failed,
                }),
                ..Self::default()
            },
            EntryKind::TurnEnd { stopped } => Self {
                kind: "turn_end",
                at,
                stopped: Some(*stopped),
                ..Self::default()
            },
            EntryKind::Compacted => Self {
                kind: "compacted",
                at,
                ..Self::default()
            },
        }
    }

    fn text(kind: &'static str, at: Option<DateTime<Utc>>, text: &str) -> Self {
        Self {
            kind,
            at,
            text: Some(text.to_owned()),
            ..Self::default()
        }
    }
}

/// The thread as text for a terminal: a head, then each entry with a mark of its kind.
#[must_use]
pub fn text(row: &ThreadRow, thread: &Thread) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} · {}",
        row.title.as_deref().unwrap_or("Untitled thread"),
        program_words(row.program)
    );
    let place = [
        row.folder.as_deref(),
        row.branch.as_deref(),
        row.model.as_deref(),
    ];
    let place: Vec<&str> = place.into_iter().flatten().collect();
    if !place.is_empty() {
        let _ = writeln!(out, "{}", place.join(" · "));
    }
    for entry in &thread.entries {
        out.push('\n');
        entry_text(&mut out, entry);
    }
    if thread.dropped > 0 {
        let _ = writeln!(out, "\n[{} more entries are not shown]", thread.dropped);
    }
    out
}

fn entry_text(out: &mut String, entry: &Entry) {
    match &entry.kind {
        EntryKind::Prompt { text, images } => {
            let _ = writeln!(out, "> You");
            indented(out, text);
            if *images > 0 {
                let _ = writeln!(out, "  [{images} image(s)]");
            }
        }
        EntryKind::Reply { text } => indented(out, text),
        EntryKind::Reasoning { text } => {
            let _ = writeln!(out, "~ Reasoning");
            indented(out, text);
        }
        EntryKind::Tool(call) => {
            let mark = if call.failed { "✗" } else { "·" };
            let _ = writeln!(out, "{mark} {} {}", call.name, call.summary);
            let body = call
                .patch
                .as_deref()
                .or(call.output.as_deref())
                .unwrap_or_default();
            let lines: Vec<&str> = body.lines().collect();
            for line in lines.iter().take(OUTPUT_LINES) {
                let _ = writeln!(out, "    {line}");
            }
            if lines.len() > OUTPUT_LINES {
                let _ = writeln!(out, "    [{} more lines]", lines.len() - OUTPUT_LINES);
            }
        }
        EntryKind::TurnEnd { stopped: true } => {
            let _ = writeln!(out, "— Stopped");
        }
        EntryKind::TurnEnd { stopped: false } => {
            let _ = writeln!(out, "— Turn ended");
        }
        EntryKind::Compacted => {
            let _ = writeln!(out, "— The earlier part of the thread was made shorter");
        }
    }
}

fn indented(out: &mut String, text: &str) {
    for line in text.lines() {
        let _ = writeln!(out, "  {line}");
    }
}
