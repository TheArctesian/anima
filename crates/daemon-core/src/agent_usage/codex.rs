//! The lines of a Codex rollout file (`~/.codex/sessions/**/rollout-*.jsonl`): the first
//! `session_meta` names the thread, its working folder and the thread that started it; each
//! `turn_context` names the model of the lines after it; a prompt is a `UserMessage` item (or
//! a `user_message` event of an older Codex); a tool call is a call item; the tokens are the
//! `last_token_usage` of each `token_count` event whose total moved.
//!
//! A sub-agent's file starts with a copy of its parent's history, before
//! `subagent_history_start_ordinal`: those lines count in the parent's file, not again here.
//! Each line is read twice, once for its kind and once for the fields of that kind, since one
//! field name holds different things in different kinds. No text is read into the counts.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::Tokens;
use super::tally::{Counts, time};

/// The response items that are a tool call.
const TOOL_CALLS: [&str; 4] = [
    "function_call",
    "custom_tool_call",
    "local_shell_call",
    "web_search_call",
];

/// The longest chain of sub-agents followed to its root thread.
const MAX_CHAIN: usize = 16;

/// What the reader keeps between the lines of one file.
#[derive(Clone, Debug, Default)]
pub(super) struct Thread {
    id: Option<String>,
    parent: Option<String>,
    history_end: Option<u64>,
    model: Option<String>,
    last_total: Option<Usage>,
    prompts_from: Option<PromptKind>,
}

/// Where a file's prompts are read from: a newer Codex writes `UserMessage` items, an older
/// one `user_message` events. The first kind seen is the one counted, so no prompt counts
/// twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptKind {
    Item,
    Event,
}

impl Thread {
    pub(super) fn ids(&self) -> Option<(&str, Option<&str>)> {
        Some((self.id.as_deref()?, self.parent.as_deref()))
    }

    fn prompt(&mut self, kind: PromptKind, counts: &mut Counts, at: chrono::DateTime<chrono::Utc>) {
        if *self.prompts_from.get_or_insert(kind) == kind {
            counts.prompt(at);
        }
    }
}

#[derive(Deserialize)]
struct Head {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    ordinal: Option<u64>,
    payload: Option<Kind>,
}

#[derive(Deserialize)]
struct Kind {
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
struct Full<P> {
    payload: P,
}

#[derive(Deserialize)]
struct Meta {
    id: Option<String>,
    cwd: Option<String>,
    parent_thread_id: Option<String>,
    subagent_history_start_ordinal: Option<u64>,
}

#[derive(Deserialize)]
struct TurnContext {
    model: Option<String>,
    cwd: Option<String>,
}

#[derive(Deserialize)]
struct TokenCount {
    info: Option<Info>,
}

#[derive(Deserialize)]
struct Info {
    total_token_usage: Option<Usage>,
    last_token_usage: Option<Usage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "the names of Codex's usage fields"
)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    cache_write_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Deserialize)]
struct ItemCompleted {
    item: Option<Kind>,
}

pub(super) fn read(thread: &mut Thread, counts: &mut Counts, line: &[u8]) {
    let Ok(head) = serde_json::from_slice::<Head>(line) else {
        return;
    };
    let payload_kind = head.payload.and_then(|payload| payload.kind);
    match head.kind.as_deref() {
        Some("session_meta") => meta(thread, counts, line),
        Some("turn_context") => {
            if let Some(context) = payload::<TurnContext>(line) {
                thread.model = context.model.or(thread.model.take());
                if let Some(cwd) = &context.cwd {
                    counts.named_folder(cwd);
                }
            }
        }
        _ => {}
    }
    let copied = matches!((thread.history_end, head.ordinal), (Some(end), Some(at)) if at < end);
    let Some(at) = head.timestamp.as_deref().and_then(time).filter(|_| !copied) else {
        return;
    };
    counts.saw(at);
    match (head.kind.as_deref(), payload_kind.as_deref()) {
        (Some("event_msg"), Some("token_count")) => tokens(thread, counts, at, line),
        (Some("event_msg"), Some("user_message")) => thread.prompt(PromptKind::Event, counts, at),
        (Some("event_msg"), Some("item_completed")) => {
            let item = payload::<ItemCompleted>(line).and_then(|done| done.item?.kind);
            if item.as_deref() == Some("UserMessage") {
                thread.prompt(PromptKind::Item, counts, at);
            }
        }
        (Some("response_item"), Some(kind)) if TOOL_CALLS.contains(&kind) => {
            counts.tool_calls(at, 1);
        }
        _ => {}
    }
}

/// The first `session_meta` of the file; a later one is a copy of the parent's.
fn meta(thread: &mut Thread, counts: &mut Counts, line: &[u8]) {
    if thread.id.is_some() {
        return;
    }
    let Some(meta) = payload::<Meta>(line) else {
        return;
    };
    thread.id = meta.id;
    thread.parent = meta.parent_thread_id;
    thread.history_end = meta.subagent_history_start_ordinal;
    if let Some(cwd) = &meta.cwd {
        counts.named_folder(cwd);
    }
}

/// The tokens of one response: the event's last usage, unless its total did not move (Codex
/// can write one event twice).
fn tokens(
    thread: &mut Thread,
    counts: &mut Counts,
    at: chrono::DateTime<chrono::Utc>,
    line: &[u8],
) {
    let Some(info) = payload::<TokenCount>(line).and_then(|count| count.info) else {
        return;
    };
    if info.total_token_usage.is_some() && info.total_token_usage == thread.last_total {
        return;
    }
    thread.last_total = info.total_token_usage;
    let Some(last) = info.last_token_usage else {
        return;
    };
    let tokens = Tokens {
        input: last
            .input_tokens
            .saturating_sub(last.cached_input_tokens)
            .saturating_sub(last.cache_write_input_tokens),
        output: last.output_tokens,
        cache_read: last.cached_input_tokens,
        cache_write: last.cache_write_input_tokens,
    };
    let model = thread.model.as_deref().unwrap_or(super::OTHER_MODEL);
    counts.tokens(at, model, tokens);
}

fn payload<P: DeserializeOwned>(line: &[u8]) -> Option<P> {
    serde_json::from_slice::<Full<P>>(line)
        .ok()
        .map(|full| full.payload)
}

/// The thread a Codex sub-agent's use counts under: the root of the chain of parents that
/// `parent_of` gives, followed at most 16 steps (a loop stops there too).
#[must_use]
pub fn root_thread<'a>(id: &'a str, parent_of: impl Fn(&str) -> Option<&'a str>) -> &'a str {
    let mut root = id;
    for _ in 0..MAX_CHAIN {
        match parent_of(root) {
            Some(parent) => root = parent,
            None => break,
        }
    }
    root
}
