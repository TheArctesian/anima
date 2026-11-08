//! What the hub keeps of a builder's log lines (BLD-048, BLD-049, D-4): each line redacted,
//! cleaned and cut to a bounded size before it is stored.
//!
//! Pure: the redaction is the event log's text rule, which reads nothing.

use super::model::FieldError;
use crate::events;

/// The most lines one report may carry.
pub(super) const MAX_LINES_PER_REPORT: usize = 1000;

/// The longest stored line in bytes; a longer one is cut at a character boundary.
const MAX_LINE_BYTES: usize = 4096;

/// The most log text kept for one job: the store drops the oldest lines past it (BLD-048).
pub(super) const MAX_JOB_LOG_BYTES: i64 = 2 * 1024 * 1024;

/// The longest failure reason a builder may give, in characters.
const MAX_REASON_CHARS: usize = 200;

/// Control characters other than tab become spaces, so no line holds a NUL (which PostgreSQL
/// text refuses), a line break or a terminal escape. Done before redaction, so a secret split
/// by a control character is still found.
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

/// One line as stored: printable, then redacted (BLD-049), then cut. Redaction comes before
/// the cut, so the cut never leaves part of a secret that redaction would have found whole.
pub(super) fn clean(line: &str) -> String {
    cut(events::redact_text(&printable(line)), MAX_LINE_BYTES)
}

/// The lines of one report as stored.
pub(super) fn clean_all(lines: &[String]) -> Result<Vec<String>, FieldError> {
    if lines.len() > MAX_LINES_PER_REPORT {
        return Err(FieldError::new(
            "logLines",
            format!("Send at most {MAX_LINES_PER_REPORT} lines in one report."),
        ));
    }
    Ok(lines.iter().map(|line| clean(line)).collect())
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
