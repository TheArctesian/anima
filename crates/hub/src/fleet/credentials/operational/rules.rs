//! What the operator may enter for an operational key (K-E5 Section 2). Pure. Each problem
//! names its field and says what to give; it never quotes the input, which may be a value
//! pasted in the wrong field (LOG-007).

use daemon_core::wire::{MAX_VALUE_BYTES, is_key_name};

use super::model::Metadata;
use crate::text::is_plain;

/// One field the operator must fix, and what to give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Problem {
    pub field: &'static str,
    pub message: &'static str,
}

const LINE_CHARS: usize = 200;
const INSTRUCTION_CHARS: usize = 4_000;
const LINK_CHARS: usize = 2_000;
const MAX_ROTATION_DAYS: i32 = 3_650;
const WORKLOAD_BYTES: usize = 40;

pub const NAME_RULE: &str =
    "Give 1 to 80 lower-case letters, digits, '_', '.' or '-'. Start with a letter.";
pub const WORKLOAD_RULE: &str =
    "Give 1 to 40 lower-case letters, digits, '_', '.' or '-'. Start with a letter.";
const LINE_RULE: &str = "Give one line of 1 to 200 characters.";
const VALUE_RULE: &str = "Give a value of 1 to 16384 bytes.";

/// Whether `text` is a workload label: `^[a-z][a-z0-9_.-]{0,39}$` (K-E5 Section 2).
#[must_use]
pub fn is_workload(text: &str) -> bool {
    text.len() <= WORKLOAD_BYTES && is_key_name(text)
}

/// Whether `text` is a name of an operational key (K-E5 Section 2).
#[must_use]
pub fn is_name(text: &str) -> bool {
    is_key_name(text)
}

/// Whether a value of `length` bytes may be stored.
#[must_use]
pub const fn value_fits(length: usize) -> bool {
    length >= 1 && length <= MAX_VALUE_BYTES
}

/// The problem of a value of `length` bytes, if any.
#[must_use]
pub fn value_problem(length: usize) -> Option<Problem> {
    (!value_fits(length)).then_some(Problem {
        field: "value",
        message: VALUE_RULE,
    })
}

/// Each problem of `metadata`, in field order; empty when it may be stored.
#[must_use]
pub fn problems(metadata: &Metadata) -> Vec<Problem> {
    let mut found = Vec::new();
    let mut check = |ok: bool, field: &'static str, message: &'static str| {
        if !ok {
            found.push(Problem { field, message });
        }
    };
    check(is_line(&metadata.provider), "provider", LINE_RULE);
    check(is_line(&metadata.purpose), "purpose", LINE_RULE);
    check(
        metadata.instructions.chars().count() <= INSTRUCTION_CHARS
            && !metadata.instructions.chars().any(is_hidden),
        "instructions",
        "Give at most 4000 characters.",
    );
    check(
        metadata.provider_link.as_deref().is_none_or(is_link),
        "providerLink",
        "Give an https address of at most 2000 characters.",
    );
    check(
        metadata
            .rotation_days
            .is_none_or(|days| (1..=MAX_ROTATION_DAYS).contains(&days)),
        "rotationDays",
        "Give 1 to 3650 days.",
    );
    found
}

fn is_line(text: &str) -> bool {
    let count = text.chars().count();
    (1..=LINE_CHARS).contains(&count) && is_plain(text) && !text.trim().is_empty()
}

/// A character instructions may not hold: a control character other than a line break or a
/// tab, or one that hides or reorders text.
fn is_hidden(c: char) -> bool {
    (c.is_control() && !matches!(c, '\n' | '\t')) || crate::text::is_invisible(c)
}

/// An `https` address with a host, no space and no control character.
fn is_link(text: &str) -> bool {
    let host = text.strip_prefix("https://").unwrap_or_default();
    text.len() <= LINK_CHARS
        && !host.is_empty()
        && !host.starts_with('/')
        && is_plain(text)
        && !text.chars().any(char::is_whitespace)
}
