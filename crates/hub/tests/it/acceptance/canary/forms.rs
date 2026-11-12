//! The forms a secret can take in a log line, and the search for them (LOG-007): the text as
//! typed, JSON-escaped, percent-encoded, hex, base64 at each byte alignment, and any 16
//! characters of its random part, so a log that keeps part of a secret is caught too.

use std::fmt::{self, Write as _};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};
use serde_json::Value;

use super::Secret;

/// The shortest run of a secret's random part that counts as the secret.
const WINDOW: usize = 16;

/// A secret found in a log line: its kind, the forms found, and the line's target and message
/// with the secret's forms cut out, so the report itself holds no secret.
pub struct Leak {
    kind: &'static str,
    forms: Vec<&'static str>,
    line: usize,
    target: String,
}

impl fmt::Display for Leak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            forms,
            line,
            target,
        } = self;
        let forms = forms.join(", ");
        write!(f, "{kind} ({forms}) on line {line}: {target}")
    }
}

/// Every form of `secret` to search for, each with its name.
fn forms(secret: &Secret) -> Vec<(&'static str, String)> {
    let text = &secret.text;
    let json = serde_json::to_string(text).unwrap();
    let mut forms = vec![
        ("raw", text.clone()),
        ("json-escaped", json[1..json.len() - 1].to_owned()),
        ("percent-encoded", percent(text, false)),
        ("percent-encoded every byte", percent(text, true)),
        (
            "percent-encoded every byte, lower case",
            percent(text, true).to_lowercase(),
        ),
        ("hex", text.bytes().fold(String::new(), hex)),
    ];
    let bytes = text.as_bytes();
    for start in 0..3.min(bytes.len()) {
        let whole = &bytes[start..start + (bytes.len() - start) / 3 * 3];
        if whole.len() >= 6 {
            forms.push(("base64", STANDARD_NO_PAD.encode(whole)));
            forms.push(("base64url", URL_SAFE_NO_PAD.encode(whole)));
        }
    }
    let random: Vec<char> = secret.random.chars().collect();
    for window in random.windows(WINDOW) {
        forms.push(("16 characters of it", window.iter().collect()));
    }
    forms
}

/// `out` with `byte` added as two lower-case hex digits.
pub fn hex(mut out: String, byte: u8) -> String {
    write!(out, "{byte:02x}").unwrap();
    out
}

/// `text` with each byte outside RFC 3986's unreserved set, or every byte, as `%XX`.
fn percent(text: &str, every: bool) -> String {
    text.bytes().fold(String::new(), |mut out, byte| {
        let unreserved = byte.is_ascii_alphanumeric() || b"-._~".contains(&byte);
        if unreserved && !every {
            out.push(char::from(byte));
        } else {
            write!(out, "%{byte:02X}").unwrap();
        }
        out
    })
}

/// Each place a form of `secret` is in `log`, one JSON event per line.
pub fn leaks(secret: &Secret, log: &str) -> Vec<Leak> {
    let forms = forms(secret);
    let mut found = Vec::new();
    for (number, line) in log.lines().enumerate() {
        let mut names: Vec<&'static str> = (forms.iter())
            .filter(|(_, needle)| line.contains(needle.as_str()))
            .map(|(name, _)| *name)
            .collect();
        names.dedup();
        if !names.is_empty() {
            found.push(Leak {
                kind: secret.kind,
                forms: names,
                line: number + 1,
                target: where_in(line, &forms),
            });
        }
    }
    found
}

/// The target and message of a log line, with every form of the secret cut out.
fn where_in(line: &str, forms: &[(&'static str, String)]) -> String {
    let event: Value = serde_json::from_str(line).unwrap_or_default();
    let mut text = format!("{} {}", event["target"], event["fields"]["message"]);
    for (_, needle) in forms {
        text = text.replace(needle.as_str(), "<secret>");
    }
    text
}
