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
    let end = text[start..]
        .find(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | ',' | '}' | '"' | '\''))
        .map_or(text.len(), |offset| start + offset);
    text[start..end]
        .find("%26")
        .map_or(end, |offset| start + offset)
}

/// `text` after the separator that follows a signature key: `=`, `:`, or either one
/// percent-encoded (`%3D`, `%3a`).
fn after_separator(text: &str) -> Option<&str> {
    text.strip_prefix(['=', ':']).or_else(|| {
        let encoded = text.get(..3)?;
        (encoded.eq_ignore_ascii_case("%3d") || encoded.eq_ignore_ascii_case("%3a"))
            .then(|| &text[3..])
    })
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
        let Some(value) = after_separator(after_key) else {
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

/// `text` with every secret that the event log's rule or the signature rule finds masked.
fn redact(text: &str) -> String {
    mask_signatures(&events::redact_text(text))
}

/// A line read two ways, both printable: as a terminal shows it (`hidden_text::visible`), and
/// spaced (`hidden_text::spaced`: each escape a space, the bytes that may end one kept).
struct Readings {
    shown: String,
    spaced: String,
}

impl Readings {
    fn of(line: &str) -> Self {
        Self {
            shown: printable(&hidden_text::visible(line)),
            spaced: printable(&hidden_text::spaced(line)),
        }
    }

    /// The shown reading redacted; or `[redacted]` whole when the line holds escapes or hidden
    /// characters and the spaced reading holds a secret. Removing an escape can take a key's
    /// first letter with it (`ESC [ p` then `assword=…`), so the shown reading alone could keep
    /// a value that the spaced reading masks (BLD-049, LOG-007).
    fn redacted(&self) -> String {
        let shown = redact(&self.shown);
        if self.shown != self.spaced && redact(&self.spaced) != self.spaced {
            REDACTED.to_owned()
        } else {
            shown
        }
    }

    /// Where the line stands to a private key block, read either way: `Some(true)` opens one,
    /// `Some(false)` closes one (or opens and closes it on one line).
    fn key_edge(&self) -> Option<bool> {
        let opens = opens_key(&self.shown) || opens_key(&self.spaced);
        let closes = closes_key(&self.shown) || closes_key(&self.spaced);
        (opens || closes).then_some(opens && !closes)
    }
}

/// One line as stored: printable, redacted (BLD-049), then cut. Redaction comes before the
/// cut, so the cut never leaves part of a secret that redaction would have found whole.
pub(super) fn clean(line: &str) -> String {
    cut(Readings::of(line).redacted(), MAX_LINE_BYTES)
}

/// Whether a line opens a private key block: PEM, OpenSSH and PGP (`-----BEGIN` with
/// `PRIVATE KEY`), SSH2 (`---- BEGIN SSH2` with `PRIVATE KEY`), or the private lines of a
/// `PuTTY` key file (`Private-Lines:`).
fn opens_key(line: &str) -> bool {
    ((line.contains("-----BEGIN") || line.contains("---- BEGIN SSH2"))
        && line.contains("PRIVATE KEY"))
        || line.contains("Private-Lines:")
}

/// Whether a line closes a private key block: `-----END`, `---- END`, or a `PuTTY` key file's
/// `Private-MAC:`.
fn closes_key(line: &str) -> bool {
    line.contains("-----END") || line.contains("---- END") || line.contains("Private-MAC:")
}

/// The lines of one report as stored, in order, and whether a private key block is still open
/// after them; `in_key` says whether one was open before them, as a block may run across
/// reports. Blank lines are dropped, and so is every line after a private key's opening line
/// up to and including its closing line. The opening line is kept as `[redacted]`.
///
/// # Errors
///
/// `logLines` for more than 1000 lines.
pub(super) fn clean_all(
    lines: &[String],
    mut in_key: bool,
) -> Result<(Vec<String>, bool), FieldError> {
    if lines.len() > MAX_LINES_PER_REPORT {
        return Err(FieldError::new(
            "logLines",
            format!("Send at most {MAX_LINES_PER_REPORT} lines in one report."),
        ));
    }
    let mut kept = Vec::with_capacity(lines.len());
    for line in lines {
        let readings = Readings::of(line);
        let edge = readings.key_edge();
        if in_key {
            in_key = edge != Some(false);
            continue;
        }
        if edge.is_some_and(|opens| opens) {
            in_key = true;
            kept.push(REDACTED.to_owned());
            continue;
        }
        let cleaned = cut(readings.redacted(), MAX_LINE_BYTES);
        if !cleaned.trim().is_empty() {
            kept.push(cleaned);
        }
    }
    Ok((kept, in_key))
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
