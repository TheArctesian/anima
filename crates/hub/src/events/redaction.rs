//! Removes secret values from an audit change before it is stored (LOG-007, D-3).

use serde_json::Value;

/// The value that replaces a secret.
pub(super) const REDACTED: &str = "[redacted]";

/// Key fragments that name a secret, compared after lower-casing and removing `_` and `-`.
/// A key that contains one (`password`, `newPassword`, `private_key`, `apiToken`) is redacted.
const SECRET_KEY_PARTS: [&str; 6] = [
    "password",
    "privatekey",
    "passphrase",
    "token",
    "secret",
    "totpcode",
];

/// Replaces the value of every object key that names a secret, at any depth and inside
/// arrays, with `"[redacted]"`.
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let value = if names_secret(&key) {
                        Value::String(REDACTED.to_owned())
                    } else {
                        redact(value)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        other => other,
    }
}

fn names_secret(key: &str) -> bool {
    let key: String = key
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect();
    SECRET_KEY_PARTS.iter().any(|part| key.contains(part))
}
