//! Label values as the text format needs them (INT-002). Every label value of the exposition
//! passes through [`label_value`], so a name an operator typed cannot break a line, close a
//! label or hide characters from whoever reads the scrape.

use crate::checks::probe::clean_message;

/// Longest label value kept, in characters: a check name's limit.
pub(super) const LABEL_MAX: usize = 200;

/// The value cleaned, then escaped for a quoted label value of the text format 0.0.4.
pub(super) fn label_value(raw: &str) -> String {
    escaped(&cleaned(raw))
}

/// Drops control and invisible characters (bidi overrides, zero-width characters, line and
/// paragraph separators) and keeps at most [`LABEL_MAX`] characters.
///
/// WP-41 T3: replace this step with `crate::text::clean` when it merges.
fn cleaned(raw: &str) -> String {
    clean_message(raw).chars().take(LABEL_MAX).collect()
}

/// Backslash, double quote and line feed escaped, as the format requires inside a label value.
pub(super) fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}
