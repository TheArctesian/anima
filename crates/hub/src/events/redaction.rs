//! Removes secret values from an audit change, and from event text, before it is stored
//! (LOG-007, D-3).
//!
//! Defence in depth: callers never put secret values in `change` or event text. It prefers
//! over-matching (`tokenExpiresAt` and `passwordChangedAt` are redacted) to letting a secret
//! through. Booleans are never redacted: they cannot hold a secret.

mod names;
#[cfg(test)]
mod tests;
mod text;

use chrono::DateTime;
use serde_json::{Map, Value};

use names::{Secrecy, normalise, secrecy};
pub(super) use text::redact_text;

/// The value that replaces a secret.
const REDACTED: &str = "[redacted]";

/// Keys whose string value names the field an object describes (`{ "field": "password", … }`,
/// JSON Patch `{ "op": …, "path": "/ssh/password", … }`). `name` is a label only beside a
/// `value` (a header `{ "name": "Authorization", "value": … }`), never on its own: a device may
/// be called `secret-lab`.
const LABELS: [&str; 5] = ["field", "path", "key", "setting", "property"];

/// Names of credential metadata kept inside a credential holder, compared normalised. Any other
/// scalar there is hidden unless it is an RFC 3339 timestamp.
const HOLDER_METADATA: [&str; 11] = [
    "kind",
    "type",
    "username",
    "id",
    "fingerprint",
    "createdat",
    "updatedat",
    "rotatedat",
    "storedat",
    "expiresat",
    "lastusedat",
];

/// Values of a key named exactly `auth` that name an authentication method, compared
/// normalised (`private_key` gives `privatekey`). Any other scalar under `auth` is a secret.
const AUTH_METHODS: [&str; 7] = [
    "password",
    "key",
    "publickey",
    "privatekey",
    "none",
    "token",
    "totp",
];

/// Replaces every secret in `value` with `"[redacted]"`, at any depth and inside arrays:
/// - the value of a key that names a secret;
/// - under a key that holds credentials, every scalar except credential metadata;
/// - in an object whose label names a secret, every sibling except the label and `op`;
/// - under a key named `auth`, everything but a method name (`{"auth": "password"}` stays);
/// - in every string, the secrets `redact_text` finds.
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(redact_fields(fields)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        Value::String(text) => Value::String(redact_text(&text)),
        other => other,
    }
}

fn redact_fields(fields: Map<String, Value>) -> Map<String, Value> {
    let header = ["name", "value"]
        .iter()
        .all(|wanted| fields.keys().any(|key| normalise(key) == *wanted));
    let secret_label = fields
        .iter()
        .find(|(key, value)| {
            is_label(key, header) && value.as_str().is_some_and(label_names_secret)
        })
        .map(|(key, _)| key.clone());
    fields
        .into_iter()
        .map(|(key, value)| {
            let value = match &secret_label {
                Some(label) if key != *label && normalise(&key) != "op" => conceal(value),
                Some(_) => redact(value),
                None if normalise(&key) == "auth" => redact_auth(value),
                None => match secrecy(&key) {
                    Secrecy::Holder => redact_holder(value),
                    Secrecy::Secret => conceal(value),
                    Secrecy::Plain => redact(value),
                },
            };
            (key, value)
        })
        .collect()
}

/// Inside a credential holder (`{"credential": {…}}`, `{"credentials": […]}`), at any depth:
/// hides every scalar except a boolean, an RFC 3339 timestamp and the scalar of a metadata
/// name, which are redacted like any other value. A secret name is hidden whatever it holds.
fn redact_holder(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let value = match secrecy(&key) {
                        Secrecy::Secret => conceal(value),
                        _ if is_metadata(&key, &value) => redact(value),
                        _ => redact_holder(value),
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(redact_holder).collect()),
        scalar => conceal(scalar),
    }
}

/// A scalar under a metadata name, or a string that is an RFC 3339 timestamp.
fn is_metadata(key: &str, value: &Value) -> bool {
    match value {
        Value::Object(_) | Value::Array(_) => false,
        Value::String(text) if DateTime::parse_from_rfc3339(text).is_ok() => true,
        _ => HOLDER_METADATA.contains(&normalise(key).as_str()),
    }
}

/// The value of a key named `auth`: a method name stays, an object or array is a credential
/// holder, any other scalar is hidden.
fn redact_auth(value: Value) -> Value {
    match value {
        Value::String(method) if AUTH_METHODS.contains(&normalise(&method).as_str()) => {
            Value::String(method)
        }
        Value::Object(_) | Value::Array(_) => redact_holder(value),
        scalar => conceal(scalar),
    }
}

fn conceal(value: Value) -> Value {
    if value.is_boolean() {
        value
    } else {
        Value::String(REDACTED.to_owned())
    }
}

fn is_label(key: &str, header: bool) -> bool {
    let key = normalise(key);
    LABELS.contains(&key.as_str()) || (header && key == "name")
}

/// A label value may be a path (`/ssh/password`, `ssh.password`, `/credentials/0/value`): it
/// names a secret if any `/` or `.` segment does (an array index or `-` never does).
fn label_names_secret(label: &str) -> bool {
    label
        .split(['/', '.'])
        .any(|segment| secrecy(segment) != Secrecy::Plain)
}
