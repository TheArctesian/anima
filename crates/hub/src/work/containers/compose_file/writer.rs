//! The writer of the Compose file that the converter makes: block YAML, with each text in
//! double quotes, so that no value can be read as a number, a truth value or a second key.

use std::fmt::Write as _;

/// One value of the file that the converter writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Value {
    Text(String),
    Truth(bool),
    Number(i64),
    List(Vec<Self>),
    /// The keys in the order in which the file has them.
    Map(Vec<(String, Self)>),
}

impl Value {
    pub(super) const fn map() -> Self {
        Self::Map(Vec::new())
    }

    pub(super) fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }

    /// The value of `key` in a map, made with `empty` when the map does not have it.
    fn at(&mut self, key: &str, empty: Self) -> &mut Self {
        if !matches!(self, Self::Map(_)) {
            *self = Self::map();
        }
        let Self::Map(entries) = self else {
            unreachable!("the value is a map")
        };
        let found = entries.iter().position(|(name, _)| name == key);
        let index = found.unwrap_or_else(|| {
            entries.push((key.to_owned(), empty));
            entries.len() - 1
        });
        &mut entries[index].1
    }

    /// Sets `key` of a map: a second value for a key takes the place of the first.
    pub(super) fn set(&mut self, key: &str, value: Self) {
        *self.at(key, Self::map()) = value;
    }

    /// The map that is the value of `key`.
    pub(super) fn child(&mut self, key: &str) -> &mut Self {
        self.at(key, Self::map())
    }

    /// Adds `value` to the list that is the value of `key`.
    pub(super) fn push(&mut self, key: &str, value: Self) {
        let list = self.at(key, Self::List(Vec::new()));
        if let Self::List(items) = list {
            items.push(value);
        } else {
            *list = Self::List(vec![value]);
        }
    }

    pub(super) fn entries_mut(&mut self) -> Option<&mut Vec<(String, Self)>> {
        match self {
            Self::Map(entries) => Some(entries),
            _ => None,
        }
    }
}

/// Whether YAML may hold `character` as it is in a text in double quotes: the printable set
/// of YAML 1.2 (`c-printable`), less the characters that a reader can take as a line break
/// (U+0085, U+2028, U+2029) or as a mark of the order of the bytes (U+FEFF). Each other
/// character is written as an escape.
const fn printable(character: char) -> bool {
    matches!(
        character,
        '\u{20}'..='\u{7E}'
            | '\u{A0}'..='\u{2027}'
            | '\u{202A}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FEFE}'
            | '\u{FF00}'..='\u{FFFD}'
            | '\u{10000}'..
    )
}

/// A text in double quotes, as YAML reads it back to the same characters.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other if printable(other) => out.push(other),
            other => {
                let _ = write!(out, "\\u{:04x}", u32::from(other));
            }
        }
    }
    out.push('"');
    out
}

/// A key as the file writes it: a fixed key of Compose as it is, each other key in quotes.
fn key(name: &str) -> String {
    let fixed = !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    let other_type = matches!(
        name,
        "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "y" | "n"
    );
    if fixed && !other_type {
        name.to_owned()
    } else {
        quoted(name)
    }
}

fn pad(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

/// What comes after `key:` or after `-`: a scalar on the same line, a list or a map below.
fn after(out: &mut String, value: &Value, indent: usize) {
    let scalar = match value {
        Value::Map(entries) if !entries.is_empty() => {
            out.push('\n');
            return map(out, entries, indent);
        }
        Value::List(items) if !items.is_empty() => {
            out.push('\n');
            return list(out, items, indent);
        }
        Value::Map(_) => "{}".to_owned(),
        Value::List(_) => "[]".to_owned(),
        Value::Text(text) => quoted(text),
        Value::Truth(truth) => truth.to_string(),
        Value::Number(number) => number.to_string(),
    };
    out.push(' ');
    out.push_str(&scalar);
    out.push('\n');
}

fn map(out: &mut String, entries: &[(String, Value)], indent: usize) {
    for (index, (name, value)) in entries.iter().enumerate() {
        // The first key of a map in a list is on the line of the `-`.
        if index > 0 || !out.ends_with("- ") {
            pad(out, indent);
        }
        out.push_str(&key(name));
        out.push(':');
        after(out, value, indent + 2);
    }
}

fn list(out: &mut String, items: &[Value], indent: usize) {
    for item in items {
        pad(out, indent);
        out.push('-');
        match item {
            Value::Map(entries) if !entries.is_empty() => {
                out.push(' ');
                map(out, entries, indent + 2);
            }
            other => after(out, other, indent + 2),
        }
    }
}

/// The YAML text of `file`, which is the map of the top level.
pub(super) fn write(file: &Value) -> String {
    let mut out = String::new();
    if let Value::Map(entries) = file {
        map(&mut out, entries, 0);
    }
    out
}
