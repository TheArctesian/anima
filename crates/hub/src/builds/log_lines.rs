//! What the hub keeps of a builder's log lines (BLD-048, BLD-049, D-4): each line redacted,
//! cleaned and cut to a bounded size before it is stored; blank lines and private key blocks
//! are not kept.
//!
//! Pure: the redaction is the event log's text rule, which reads nothing.

use super::{hidden_text, model::FieldError, private_keys};
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

/// What a private key block's opening line becomes; the block's other lines are not stored.
pub(super) const KEY_BLOCK: &str = "[redacted: private key block]";

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

/// `text` with every secret masked that the event log's rule, the signature rule or the
/// minisign rule finds; `[redacted]` whole for private key text. A minisign public key's id is
/// kept: the build log may show the public key.
fn redact(text: &str) -> String {
    if private_keys::is_secret_text(text) {
        return REDACTED.to_owned();
    }
    if let Some(colon) = private_keys::public_key_id_colon(text) {
        return format!("{}:{}", redact(&text[..colon]), redact(&text[colon + 1..]));
    }
    mask_signatures(&events::redact_text(text))
}

/// The values that redaction replaced in `text`, given `masked = redact(text)`. `None` when
/// they cannot be told apart from the text around them (the text holds `[redacted]` itself).
fn masked_values<'a>(text: &'a str, masked: &str) -> Option<Vec<&'a str>> {
    let mut pieces = masked.split(REDACTED);
    let first = pieces.next().unwrap_or_default();
    let mut rest = text.strip_prefix(first)?;
    let mut values = Vec::new();
    let mut pieces = pieces.peekable();
    while let Some(piece) = pieces.next() {
        let end = if pieces.peek().is_none() {
            rest.strip_suffix(piece)?.len()
        } else if piece.is_empty() {
            return None;
        } else {
            rest.find(piece)?
        };
        values.push(&rest[..end]);
        rest = &rest[end + piece.len()..];
    }
    Some(values)
}

/// What `line` stores of the secrets that `reading` finds, given the stored text: `true` when
/// one of them, or the whole reading, would be kept.
fn leaks(reading: &str, stored: &str) -> bool {
    let masked = redact(reading);
    if masked == reading {
        return false;
    }
    if masked == REDACTED {
        return stored != REDACTED;
    }
    masked_values(reading, &masked).is_none_or(|values| {
        values
            .iter()
            .any(|value| !value.is_empty() && stored.contains(value))
    })
}

/// A line read four ways, all printable: as a terminal shows it (`hidden_text::visible`), and
/// three stricter ways that keep what an escape may have taken from a key (`spaced`, `joined`,
/// `joined_rare`).
struct Readings {
    shown: String,
    stricter: [String; 3],
}

impl Readings {
    fn of(line: &str) -> Self {
        Self {
            shown: printable(&hidden_text::visible(line)),
            stricter: [
                printable(&hidden_text::spaced(line)),
                printable(&hidden_text::joined(line)),
                printable(&hidden_text::joined_rare(line)),
            ],
        }
    }

    fn all(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.shown.as_str()).chain(self.stricter.iter().map(String::as_str))
    }

    /// The shown reading redacted; or `[redacted]` whole when it would keep a value that a
    /// stricter reading masks. Removing an escape can take a key's first letter with it
    /// (`ESC [ p` then `assword=…`), so the shown reading alone could keep a secret
    /// (BLD-049, LOG-007).
    fn redacted(&self) -> String {
        let shown = redact(&self.shown);
        let leaked = self
            .stricter
            .iter()
            .any(|reading| *reading != self.shown && leaks(reading, &shown));
        if leaked { REDACTED.to_owned() } else { shown }
    }

    /// How many lines after this one belong to the key block it opens, read any way.
    fn opens_key(&self) -> Option<u16> {
        self.all().filter_map(private_keys::opens).max()
    }

    fn closes_key(&self) -> bool {
        self.all().any(private_keys::closes)
    }

    fn is_key_material(&self) -> bool {
        self.all().any(private_keys::is_material)
    }
}

/// One line as stored: printable, redacted (BLD-049), then cut. Redaction comes before the
/// cut, so the cut never leaves part of a secret that redaction would have found whole.
pub(super) fn clean(line: &str) -> String {
    cut(Readings::of(line).redacted(), MAX_LINE_BYTES)
}

/// The lines of one report as stored, in order, and the key block still open after them: how
/// many more lines it may take. `open` is the block open before them, as a block may run
/// across reports. Blank lines are dropped. A private key's opening line is stored as
/// [`KEY_BLOCK`]; the lines after it are dropped up to and including its closing line, while
/// they are key material and up to the block's line limit. The first other line ends the
/// block and is kept, so an opening line with no key after it costs the log nothing more.
///
/// # Errors
///
/// `logLines` for more than 1000 lines.
pub(super) fn clean_all(
    lines: &[String],
    mut open: Option<u16>,
) -> Result<(Vec<String>, Option<u16>), FieldError> {
    if lines.len() > MAX_LINES_PER_REPORT {
        return Err(FieldError::new(
            "logLines",
            format!("Send at most {MAX_LINES_PER_REPORT} lines in one report."),
        ));
    }
    let mut kept = Vec::with_capacity(lines.len());
    for line in lines {
        let readings = Readings::of(line);
        if let Some(left) = open.take() {
            if readings.closes_key() {
                continue;
            }
            if left > 0 && readings.is_key_material() {
                open = Some(left - 1);
                continue;
            }
        }
        if let Some(block) = readings.opens_key() {
            open = (!readings.closes_key()).then_some(block);
            kept.push(KEY_BLOCK.to_owned());
            continue;
        }
        let cleaned = cut(readings.redacted(), MAX_LINE_BYTES);
        if !cleaned.trim().is_empty() {
            kept.push(cleaned);
        }
    }
    Ok((kept, open))
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
