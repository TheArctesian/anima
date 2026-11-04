//! Removes secret values from an audit change before it is stored (LOG-007, D-3).
//!
//! A backstop: callers should not pass secrets. It prefers over-matching (`tokenExpiresAt` and
//! `passwordChangedAt` are redacted) to letting a secret through. Booleans are never redacted:
//! they cannot hold a secret.

use serde_json::{Map, Value};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

#[cfg(test)]
mod tests;

/// The value that replaces a secret.
const REDACTED: &str = "[redacted]";

/// A normalised name that contains one of these names a secret.
const SECRET_PARTS: [&str; 13] = [
    "password",
    "passwd",
    "passphrase",
    "secret",
    "token",
    "credential",
    "authorization",
    "cookie",
    "bearer",
    "privatekey",
    "keymaterial",
    "totp",
    "otpauth",
];

/// A name with one of these as a whole word names a secret (`otpCode`, `sshKeyPem`, `sshPass`),
/// while `notPaused`, `footprint` and `hotplug` do not.
const SECRET_WORDS: [&str; 6] = ["otp", "pem", "auth", "pin", "pass", "pwd"];

/// Names written as one lower-case word that end in "key". A name whose last word is `key`
/// (`apiKey`, `ssh_key`, `APIKeys`) is a secret too; `monkey`, `keyboard` and `keyId` are not.
const KEY_COMPOUNDS: [&str; 5] = [
    "apikey",
    "sshkey",
    "accesskey",
    "signingkey",
    "encryptionkey",
];

/// Public keys, never secret. The host-key fingerprint is audit evidence (SEC-006).
const PUBLIC_KEYS: [&str; 3] = ["hostkey", "publickey", "nodekey"];

/// Keys whose string value names the field an object describes (`{ "field": "password", … }`,
/// JSON Patch `{ "op": …, "path": "/ssh/password", … }`).
const LABELS: [&str; 5] = ["field", "path", "key", "setting", "property"];

/// Replaces every secret in `value` with `"[redacted]"`, at any depth and inside arrays:
/// - the value of a key that names a secret;
/// - in an object whose label names a secret, every sibling except the label and `op`;
/// - any string that holds a PEM private key or an `otpauth://` URI.
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(redact_fields(fields)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        Value::String(text) if is_secret_text(&text) => Value::String(REDACTED.to_owned()),
        other => other,
    }
}

fn redact_fields(fields: Map<String, Value>) -> Map<String, Value> {
    let secret_label = fields
        .iter()
        .find(|(key, value)| is_label(key) && value.as_str().is_some_and(label_names_secret))
        .map(|(key, _)| key.clone());
    fields
        .into_iter()
        .map(|(key, value)| {
            let hidden = match &secret_label {
                Some(label) => key != *label && normalise(&key) != "op",
                None => names_secret(&key),
            };
            let value = if hidden {
                conceal(value)
            } else {
                redact(value)
            };
            (key, value)
        })
        .collect()
}

fn conceal(value: Value) -> Value {
    if value.is_boolean() {
        value
    } else {
        Value::String(REDACTED.to_owned())
    }
}

fn is_label(key: &str) -> bool {
    LABELS.contains(&normalise(key).as_str())
}

/// A label value may be a path (`/ssh/password`, `ssh.password`); its last segment counts.
fn label_names_secret(label: &str) -> bool {
    label.rsplit(['/', '.']).next().is_some_and(names_secret)
}

fn is_secret_text(text: &str) -> bool {
    let start = text.trim_start();
    (text.contains("-----BEGIN") && text.contains("PRIVATE KEY"))
        || start
            .get(..10)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("otpauth://"))
}

fn names_secret(name: &str) -> bool {
    let words = words(name);
    let joined = words.concat();
    let has_word = |wanted: &str| words.iter().any(|word| word == wanted);
    !PUBLIC_KEYS.contains(&joined.as_str())
        && (SECRET_PARTS.iter().any(|part| joined.contains(part))
            || SECRET_WORDS.iter().any(|word| has_word(word))
            || words.last().is_some_and(|word| word == "key")
            || KEY_COMPOUNDS
                .iter()
                .any(|compound| joined.ends_with(compound)))
}

/// The name in lower-case ASCII letters and digits only.
fn normalise(name: &str) -> String {
    words(name).concat()
}

/// Splits a name into lower-case ASCII words. NFKD with combining marks dropped turns `pässword`
/// into `password` and full-width letters into ASCII. Any other character (separators,
/// zero-width spaces, other scripts) ends a word, and so does a case change: `apiKey`, `api_key`
/// and `APIKey` all give `["api", "key"]`. A plural `s` on the last word is dropped (`apiKeys`),
/// but not a double `ss` (`pass`).
fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.nfkd().filter(|c| !is_combining_mark(*c)).collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut after_lower = false;
    for (index, &c) in chars.iter().enumerate() {
        let before_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
        if c.is_uppercase() && (after_lower || before_lower) && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        let folded: String = c
            .to_lowercase()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        if folded.is_empty() && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        word.push_str(&folded);
        after_lower = !folded.is_empty() && !c.is_uppercase();
    }
    if !word.is_empty() {
        words.push(word);
    }
    if let Some(last) = words
        .last_mut()
        .filter(|last| last.ends_with('s') && !last.ends_with("ss"))
    {
        last.pop();
    }
    words
}
