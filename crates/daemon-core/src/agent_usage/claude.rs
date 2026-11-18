//! The lines of a Claude Code session file (`~/.claude/projects/<project>/<id>.jsonl` and its
//! sub-agents' `<id>/subagents/**/*.jsonl`): a prompt is a `user` line the operator typed, not
//! a sub-agent's; the tool calls and tokens are in the `assistant` lines, the sub-agents' too.
//!
//! Claude Code writes one `assistant` line for each block of a reply, each with the usage of
//! the whole reply, so the tokens of a reply count once, by its `message.id`. Only the fields
//! named below are read; serde skips the rest, the text included.

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use serde::Deserialize;

use super::Tokens;
use super::content::Content;
use super::tally::{Counts, time};

/// How many reply IDs a file remembers: the lines of one reply follow each other, so a few
/// suffice, and memory stays bounded however long the file.
const REMEMBERED: usize = 16;

/// The model Claude Code names on a reply it made itself, with no model call and no tokens.
const SYNTHETIC: &str = "<synthetic>";

/// The hashes of the replies whose tokens were counted last.
#[derive(Clone, Debug, Default)]
pub(super) struct Recent(VecDeque<u64>);

impl Recent {
    /// Whether the reply `id` is new; remembers it.
    fn first(&mut self, id: &str) -> bool {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        let hash = hasher.finish();
        if self.0.contains(&hash) {
            return false;
        }
        if self.0.len() == REMEMBERED {
            self.0.pop_front();
        }
        self.0.push_back(hash);
        true
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    is_sidechain: Option<bool>,
    is_meta: Option<bool>,
    is_compact_summary: Option<bool>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    content: Option<Content>,
}

#[derive(Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "the names of Claude Code's usage fields"
)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

pub(super) fn read(recent: &mut Recent, counts: &mut Counts, line: &[u8]) {
    let Ok(line) = serde_json::from_slice::<Line>(line) else {
        return;
    };
    if let Some(cwd) = &line.cwd {
        counts.named_folder(cwd);
    }
    let Some(at) = line.timestamp.as_deref().and_then(time) else {
        return;
    };
    counts.saw(at);
    let Some(message) = line.message else {
        return;
    };
    match line.kind.as_deref() {
        Some("user") => {
            let program_wrote = line.is_sidechain == Some(true)
                || line.is_meta == Some(true)
                || line.is_compact_summary == Some(true);
            if !program_wrote && message.content.is_some_and(|content| content.typed) {
                counts.prompt(at);
            }
        }
        Some("assistant") => reply(recent, counts, at, message),
        _ => {}
    }
}

/// The tool calls of one `assistant` line, and the tokens of its reply the first time the
/// reply is seen.
fn reply(
    recent: &mut Recent,
    counts: &mut Counts,
    at: chrono::DateTime<chrono::Utc>,
    message: Message,
) {
    let Some(model) = message.model.as_deref().filter(|model| *model != SYNTHETIC) else {
        return;
    };
    counts.tool_calls(at, message.content.map_or(0, |content| content.tool_calls));
    let Some(usage) = message.usage else {
        return;
    };
    if message.id.as_deref().is_some_and(|id| !recent.first(id)) {
        return;
    }
    let tokens = Tokens {
        input: usage.input_tokens.unwrap_or_default(),
        output: usage.output_tokens.unwrap_or_default(),
        cache_read: usage.cache_read_input_tokens.unwrap_or_default(),
        cache_write: usage.cache_creation_input_tokens.unwrap_or_default(),
    };
    counts.tokens(at, model, tokens);
}
