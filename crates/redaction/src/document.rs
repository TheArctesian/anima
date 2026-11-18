//! Secrets in a text of many lines: a session log, a note, an upload (RED-001, COL-005).

use super::{REDACTED, text::redact_text};

/// A key file that spans lines: the texts that all stand in its first line, and the text that
/// stands in its last line.
const KEY_BLOCKS: [(&[&str], &str); 2] = [
    (&["-----BEGIN", "PRIVATE KEY"], "-----END"),
    (&["PuTTY-User-Key-File-"], "Private-MAC:"),
];

/// `text` with each line redacted as [`redact_text`] redacts it, and each private key block
/// replaced by one `"[redacted]"` line, so that one key does not hide the whole text. A block
/// with no last line is cut by a clip: all text after its first line is removed. Line endings
/// stay as they are.
#[must_use]
pub fn redact_document(text: &str) -> String {
    let mut redacted = String::with_capacity(text.len());
    let mut block_end: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        if let Some(end) = block_end {
            if body.contains(end) {
                block_end = None;
            }
            continue;
        }
        match block_start(body) {
            Some(end) => {
                redacted.push_str(REDACTED);
                block_end = (!body.contains(end)).then_some(end);
            }
            None => redacted.push_str(&redact_text(body)),
        }
        redacted.push_str(ending);
    }
    redacted
}

/// The text that ends the key block that `line` starts.
fn block_start(line: &str) -> Option<&'static str> {
    KEY_BLOCKS
        .iter()
        .find(|(starts, _)| starts.iter().all(|start| line.contains(start)))
        .map(|(_, end)| *end)
}
