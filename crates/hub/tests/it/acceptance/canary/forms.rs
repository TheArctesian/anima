//! The forms a secret can take in a log line, and the search for them (LOG-007): the text as
//! typed, JSON-escaped, percent-encoded, hex, base64 at each byte alignment, and any 12
//! characters of its random part, so a log that keeps part of a secret is caught too. A code of
//! digits is searched as a number, with no digit on either side, in a message or a field. That
//! search has a small chance of a false hit on a hub number that equals the code, and a failure
//! of that kind cannot be reproduced, because the code changes with each run.

use std::fmt::{self, Write as _};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};
use serde_json::Value;

use super::Secret;

/// The shortest run of a secret's random part that counts as the secret.
const WINDOW: usize = 12;

/// The fields of a sqlx event that sqlx fills with its own digits: elapsed times.
const SQLX_ELAPSED: [&str; 2] = ["elapsed", "elapsed_secs"];

/// A secret found in a log line: its kind, the forms found, the line number and the line's
/// target. The report holds no text of the line: a message can hold another secret, or a form
/// of this one the search does not cover.
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
    if secret.digits {
        let mut forms = vec![
            ("as a number", text.clone()),
            ("hex", text.bytes().fold(String::new(), hex)),
        ];
        // A number field drops leading zeros; four digits or fewer would match too much.
        let trimmed = text.trim_start_matches('0');
        if trimmed.len() < text.len() && trimmed.len() >= 5 {
            forms.push(("as a number without its leading zero", trimmed.to_owned()));
        }
        return forms;
    }
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
        forms.push(("12 characters of it", window.iter().collect()));
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
        let searched = if secret.digits {
            without_own_digits(line)
        } else {
            line.to_owned()
        };
        let mut names: Vec<&'static str> = (forms.iter())
            .filter(|(_, needle)| holds(&searched, needle, secret.digits))
            .map(|(name, _)| *name)
            .collect();
        names.dedup();
        if !names.is_empty() {
            found.push(Leak {
                kind: secret.kind,
                forms: names,
                line: number + 1,
                target: target_of(line),
            });
        }
    }
    found
}

/// Whether `line` holds `needle`; with `digits`, only where no digit is on either side.
fn holds(line: &str, needle: &str, digits: bool) -> bool {
    let digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
    line.match_indices(needle).any(|(at, _)| {
        let (before, after) = (
            line[..at].chars().next_back(),
            line[at + needle.len()..].chars().next(),
        );
        !digits || (!digit(before) && !digit(after))
    })
}

/// The event of `line` without the digits the subscriber and sqlx write of their own, which no
/// secret reaches: the top-level `timestamp`, `span.request_id`, and a sqlx event's elapsed
/// times. A field of the same name anywhere else is kept and searched.
fn without_own_digits(line: &str) -> String {
    let mut event: Value = serde_json::from_str(line).unwrap_or_default();
    let from_sqlx = event["target"]
        .as_str()
        .is_some_and(|t| t.starts_with("sqlx"));
    if let Some(fields) = event.as_object_mut() {
        fields.remove("timestamp");
    }
    if let Some(span) = event["span"].as_object_mut() {
        span.remove("request_id");
    }
    if from_sqlx && let Some(fields) = event["fields"].as_object_mut() {
        for name in SQLX_ELAPSED {
            fields.remove(name);
        }
    }
    event.to_string()
}

/// The target of a log line: a module path, never a value.
fn target_of(line: &str) -> String {
    let event: Value = serde_json::from_str(line).unwrap_or_default();
    event["target"].as_str().unwrap_or("[no target]").to_owned()
}
