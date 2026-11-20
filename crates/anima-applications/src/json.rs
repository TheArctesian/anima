//! The writer of the final form (contract K10 Section 6.4.4, step 5) and of the records of
//! the program: JSON that a second reader cannot read in a different way.

use std::fmt::Write;

/// One value of the final form. A number and a truth value are written from their own typed
/// values, never from text of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Json {
    Truth(bool),
    Whole(u64),
    /// Billionths, written as a decimal number (`cpus`).
    Billionths(u64),
    Text(String),
    List(Vec<Json>),
    /// The members in their order.
    Record(Vec<(String, Json)>),
}

impl Json {
    pub(crate) fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }

    pub(crate) const fn empty() -> Self {
        Self::Record(Vec::new())
    }
}

/// True for a character that the final form writes as `\uXXXX`: each below U+0020, U+007F to
/// U+009F, U+2028, U+2029 and U+FEFF.
const fn is_escaped(character: char) -> bool {
    matches!(character, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}' | '\u{feff}')
}

/// Writes one string in quotes. In a value, each `$` is written `$$`, so that Compose fills
/// no variable. A character above U+FFFF is written as itself, so no pair of surrogates is
/// written.
fn string(out: &mut String, text: &str, value: bool) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' if value => out.push_str("$$"),
            escaped if is_escaped(escaped) => {
                // A write to a `String` does not fail.
                let _ = write!(out, "\\u{:04x}", u32::from(escaped));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn billionths(out: &mut String, value: u64) {
    let (units, part) = (value / 1_000_000_000, value % 1_000_000_000);
    let part = format!("{part:09}");
    let part = part.trim_end_matches('0');
    let _ = write!(out, "{units}.{}", if part.is_empty() { "0" } else { part });
}

fn write(out: &mut String, json: &Json) {
    match json {
        Json::Truth(truth) => out.push_str(if *truth { "true" } else { "false" }),
        Json::Whole(whole) => {
            let _ = write!(out, "{whole}");
        }
        Json::Billionths(value) => billionths(out, *value),
        Json::Text(text) => string(out, text, true),
        Json::List(values) => {
            out.push('[');
            for (at, value) in values.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                write(out, value);
            }
            out.push(']');
        }
        Json::Record(members) => {
            out.push('{');
            for (at, (name, value)) in members.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                string(out, name, false);
                out.push(':');
                write(out, value);
            }
            out.push('}');
        }
    }
}

/// The text of `json`, with a line end after it.
pub(crate) fn written(json: &Json) -> String {
    let mut out = String::new();
    write(&mut out, json);
    out.push('\n');
    out
}
