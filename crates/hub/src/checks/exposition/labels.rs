//! Label values as the text format needs them (INT-002). Every label value of the exposition
//! passes through [`label_value`], so a name an operator typed cannot break a line, close a
//! label or hide characters from whoever reads the scrape.

use crate::text;

/// Longest label value kept, in characters: a check name's limit.
pub(super) const LABEL_MAX: usize = 200;

/// The value without control or invisible characters (`text::clean`), cut to [`LABEL_MAX`]
/// characters, then escaped for a quoted label value of the text format 0.0.4.
pub(super) fn label_value(raw: &str) -> String {
    escaped(&text::clean(raw, LABEL_MAX))
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
