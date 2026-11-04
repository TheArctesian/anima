//! Removes secret values from an audit change before it is stored (LOG-007, D-3).
//!
//! A backstop: callers should not pass secrets. It prefers over-matching (`hasPassword`,
//! `hostKey` and `credentialStored` are redacted) to letting a secret through.

use serde_json::{Map, Value};
use unicode_normalization::UnicodeNormalization;

/// The value that replaces a secret.
const REDACTED: &str = "[redacted]";

/// A normalised name that contains one of these names a secret.
const SECRET_PARTS: [&str; 12] = [
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
    "totp",
    "otp",
];

/// A normalised name equal to one of these names a secret.
const SECRET_NAMES: [&str; 3] = ["pass", "pwd", "key"];

/// Names written as one lower-case word that end in "key". A name whose last word is `key`
/// (`apiKey`, `ssh_key`, `APIKey`) is a secret too; `monkey` and `keyboard` are not.
const KEY_COMPOUNDS: [&str; 5] = [
    "apikey",
    "sshkey",
    "accesskey",
    "signingkey",
    "encryptionkey",
];

/// In an object whose `field` or `name` value names a secret, these sibling values are redacted.
const PAIRED_VALUES: [&str; 5] = ["value", "from", "to", "old", "new"];

/// Replaces the value of every object key that names a secret, at any depth and inside
/// arrays, with `"[redacted]"`; also the values of a `{ field, from, to }`-style pair whose
/// field names a secret.
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(redact_fields(fields)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        other => other,
    }
}

fn redact_fields(fields: Map<String, Value>) -> Map<String, Value> {
    let labels_secret = fields.iter().any(|(key, value)| {
        matches!(normalise(key).as_str(), "field" | "name")
            && value.as_str().is_some_and(names_secret)
    });
    fields
        .into_iter()
        .map(|(key, value)| {
            let paired = labels_secret && PAIRED_VALUES.contains(&normalise(&key).as_str());
            let value = if paired || names_secret(&key) {
                Value::String(REDACTED.to_owned())
            } else {
                redact(value)
            };
            (key, value)
        })
        .collect()
}

fn names_secret(name: &str) -> bool {
    let words = words(name);
    let joined = words.concat();
    SECRET_PARTS.iter().any(|part| joined.contains(part))
        || SECRET_NAMES.contains(&joined.as_str())
        || words.last().is_some_and(|word| word == "key")
        || KEY_COMPOUNDS
            .iter()
            .any(|compound| joined.ends_with(compound))
}

/// The name in lower-case ASCII letters and digits only.
fn normalise(name: &str) -> String {
    words(name).concat()
}

/// Splits an NFKC-normalised name into lower-case ASCII words. Any other character
/// (separators, zero-width spaces, other scripts) ends a word, and so does a case change:
/// `apiKey`, `api_key` and `APIKey` all give `["api", "key"]`.
fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.nfkc().collect();
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
    words
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::redact;

    fn kept(change: &Value) {
        assert_eq!(&redact(change.clone()), change);
    }

    #[test]
    fn log_007_redaction_replaces_secret_keys_at_any_depth_and_inside_arrays() {
        let change = json!({
            "Password": "a",
            "name": "lab-router",
            "ssh": { "PRIVATE_KEY": "b", "privateKey": "c", "passphrase": "d", "port": 22 },
            "steps": [{ "totpCode": "123456" }, { "otp": "654321" }, [{ "Token": "e" }]],
            "apiSecret": { "nested": "f" },
            "newPassword": "g",
            "headers": { "Authorization": "Bearer x", "Cookie": "y", "passwd": "z" },
            "keys": { "apiKey": "k1", "APIKey": "k2", "ssh_key": "k3", "sshkey": "k4", "key": "k5" },
            "short": { "pass": "p", "PWD": "q", "credential": "r" },
        });

        assert_eq!(
            redact(change),
            json!({
                "Password": "[redacted]",
                "name": "lab-router",
                "ssh": {
                    "PRIVATE_KEY": "[redacted]",
                    "privateKey": "[redacted]",
                    "passphrase": "[redacted]",
                    "port": 22,
                },
                "steps": [{ "totpCode": "[redacted]" }, { "otp": "[redacted]" }, [{ "Token": "[redacted]" }]],
                "apiSecret": "[redacted]",
                "newPassword": "[redacted]",
                "headers": { "Authorization": "[redacted]", "Cookie": "[redacted]", "passwd": "[redacted]" },
                "keys": {
                    "apiKey": "[redacted]",
                    "APIKey": "[redacted]",
                    "ssh_key": "[redacted]",
                    "sshkey": "[redacted]",
                    "key": "[redacted]",
                },
                "short": { "pass": "[redacted]", "PWD": "[redacted]", "credential": "[redacted]" },
            })
        );
    }

    #[test]
    fn log_007_redaction_sees_through_zero_width_and_full_width_characters() {
        let change = json!({
            "pass\u{200b}word": "a",
            "\u{ff30}\u{ff41}\u{ff53}\u{ff53}\u{ff57}\u{ff4f}\u{ff52}\u{ff44}": "b",
            "to\u{200d}ken": "c",
        });

        assert_eq!(
            redact(change),
            json!({
                "pass\u{200b}word": "[redacted]",
                "\u{ff30}\u{ff41}\u{ff53}\u{ff53}\u{ff57}\u{ff4f}\u{ff52}\u{ff44}": "[redacted]",
                "to\u{200d}ken": "[redacted]",
            })
        );
    }

    #[test]
    fn log_007_redaction_hides_the_values_of_a_field_that_names_a_secret() {
        let change = json!([
            { "field": "password", "from": "a", "to": "b" },
            { "name": "API_TOKEN", "value": "c", "old": "d", "new": "e", "at": "noon" },
            { "field": "port", "from": 22, "to": 2222 },
        ]);

        assert_eq!(
            redact(change),
            json!([
                { "field": "password", "from": "[redacted]", "to": "[redacted]" },
                {
                    "name": "API_TOKEN",
                    "value": "[redacted]",
                    "old": "[redacted]",
                    "new": "[redacted]",
                    "at": "noon",
                },
                { "field": "port", "from": 22, "to": 2222 },
            ])
        );
    }

    #[test]
    fn log_007_redaction_keeps_values_without_secret_keys() {
        kept(&json!(["password", { "username": "admin", "port": 22, "tags": ["token"] }, null]));
        kept(&json!({ "keyboard": "us", "monkey": 1, "name": "lab-router", "deviceName": "x" }));
        kept(&json!({ "field": "name", "from": "lab-router", "to": "core-router" }));
    }

    #[test]
    fn log_007_redaction_over_matches_deliberately() {
        let change =
            json!({ "hasPassword": true, "hostKey": "SHA256:abc", "credentialStored": true });
        assert_eq!(
            redact(change),
            json!({ "hasPassword": "[redacted]", "hostKey": "[redacted]", "credentialStored": "[redacted]" })
        );
    }
}
