//! What the hub keeps of a builder's log lines (BLD-048, BLD-049, D-4): each line redacted,
//! cleaned and cut to a bounded size before it is stored; blank lines and private key blocks
//! are not kept.
//!
//! Pure: the redaction is the event log's text rule, which reads nothing.

use super::{hidden_text, model::FieldError};
use crate::events;

/// The most lines one report may carry.
pub(super) const MAX_LINES_PER_REPORT: usize = 1000;

/// The longest stored line in bytes; a longer one is cut at a character boundary.
const MAX_LINE_BYTES: usize = 4096;

/// The most log kept for one job: the store drops the oldest lines past it (BLD-048).
pub(super) const MAX_JOB_LOG_BYTES: i64 = 2 * 1024 * 1024;

/// What each stored line counts towards [`MAX_JOB_LOG_BYTES`] besides its text, so many short
/// lines cannot keep an unbounded number of rows (at most 2 MiB / 64 = 32768 rows a job).
pub(super) const LINE_OVERHEAD_BYTES: i64 = 64;

/// The longest failure reason a builder may give, in characters.
const MAX_REASON_CHARS: usize = 200;

/// What a masked value becomes, as in the event log.
const REDACTED: &str = "[redacted]";

/// A key ending in this, ignoring ASCII case, holds a signature: `X-Amz-Signature=<hex>` in a
/// presigned URL lets anyone write the object until it expires (T3).
const SIGNATURE: &str = "signature";

/// Control characters other than tab become spaces, so no line holds a NUL (which PostgreSQL
/// text refuses) or a line break.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() && c != '\t' { ' ' } else { c })
        .collect()
}

/// `text` cut to at most `max` bytes at a character boundary.
fn cut(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// The byte where the value after a signature key ends: up to whitespace, `&`, `;`, `,`, `}`
/// or a quote.
fn value_end(text: &str, start: usize) -> usize {
    text[start..]
        .find(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | ',' | '}' | '"' | '\''))
        .map_or(text.len(), |offset| start + offset)
}

/// `text` with the value of every `…signature=value` and `…signature: value` masked.
fn mask_signatures(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut masked = String::with_capacity(text.len());
    let (mut copied, mut index) = (0, 0);
    while let Some(offset) = lower[index..].find(SIGNATURE) {
        index += offset + SIGNATURE.len();
        let rest = &text[index..];
        let after_key = rest.trim_start_matches(['"', '\'']).trim_start();
        let Some(value) = after_key.strip_prefix(['=', ':']) else {
            continue;
        };
        let start = text.len() - value.trim_start_matches([' ', '"', '\'']).len();
        let end = value_end(text, start);
        if end > start {
            masked.push_str(&text[copied..start]);
            masked.push_str(REDACTED);
            (copied, index) = (end, end);
        }
    }
    masked.push_str(&text[copied..]);
    masked
}

/// One line as stored: what a terminal shows of it, printable, then redacted (BLD-049), then
/// cut. Redaction comes before the cut, so the cut never leaves part of a secret that
/// redaction would have found whole.
pub(super) fn clean(line: &str) -> String {
    let shown = printable(&hidden_text::visible(line));
    cut(
        mask_signatures(&events::redact_text(&shown)),
        MAX_LINE_BYTES,
    )
}

/// Where a cleaned line stands to a PEM private key block.
fn key_edge(line: &str) -> Option<bool> {
    if !line.contains("PRIVATE KEY-----") {
        None
    } else if line.contains("-----BEGIN") {
        Some(!line.contains("-----END"))
    } else {
        line.contains("-----END").then_some(false)
    }
}

/// The lines of one report as stored, in order: blank lines are dropped, and so is every line
/// from a private key's `BEGIN` line to its `END` line (the `BEGIN` line is kept redacted). A
/// block that runs past the report's end drops the rest of the report.
pub(super) fn clean_all(lines: &[String]) -> Result<Vec<String>, FieldError> {
    if lines.len() > MAX_LINES_PER_REPORT {
        return Err(FieldError::new(
            "logLines",
            format!("Send at most {MAX_LINES_PER_REPORT} lines in one report."),
        ));
    }
    let mut in_key = false;
    let mut kept = Vec::with_capacity(lines.len());
    for line in lines {
        let shown = printable(&hidden_text::visible(line));
        let was_in_key = in_key;
        match key_edge(&shown) {
            Some(opens) => in_key = opens,
            None if in_key => continue,
            None => {}
        }
        let cleaned = clean(line);
        if !was_in_key && !cleaned.trim().is_empty() {
            kept.push(cleaned);
        }
    }
    Ok(kept)
}

/// A builder's failure reason as stored: as a log line, at most 200 characters, and never
/// empty.
pub(super) fn reason(text: Option<&str>) -> String {
    let cleaned = clean(text.unwrap_or_default());
    let trimmed: String = cleaned.trim().chars().take(MAX_REASON_CHARS).collect();
    if trimmed.is_empty() {
        "the builder reported a failure".to_owned()
    } else {
        trimmed
    }
}
