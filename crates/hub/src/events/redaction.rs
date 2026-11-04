//! Removes secret values from an audit change, and from event text, before it is stored
//! (LOG-007, D-3).
//!
//! Defence in depth: callers never put secret values in `change` or event text. It prefers over-matching
//! (`tokenExpiresAt` and `passwordChangedAt` are redacted) to letting a secret through.
//! Booleans are never redacted: they cannot hold a secret.

mod names;
#[cfg(test)]
mod tests;

use chrono::DateTime;
use serde_json::{Map, Value};

use names::{Secrecy, normalise, secrecy};

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

/// Text that marks a whole string as a secret, compared case-sensitively.
const SECRET_MARKERS: [&str; 1] = ["PuTTY-User-Key-File-"];

/// Text that marks a whole string as a secret, compared ignoring ASCII case.
const SECRET_MARKERS_ANY_CASE: [&str; 2] = ["otpauth://", "authorization:"];

/// Prefixes of an HTTP credential, compared ignoring ASCII case and followed by whitespace.
const CREDENTIAL_SCHEMES: [&str; 2] = ["bearer", "basic"];

/// Replaces every secret in `value` with `"[redacted]"`, at any depth and inside arrays:
/// - the value of a key that names a secret;
/// - under a key that holds credentials, every scalar except credential metadata;
/// - in an object whose label names a secret, every sibling except the label and `op`;
/// - any string that holds a private key, an `otpauth://` URI or an HTTP credential;
/// - the password in a URL (`scheme://user:[redacted]@host`).
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(redact_fields(fields)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        Value::String(text) => Value::String(redact_text(&text)),
        other => other,
    }
}

/// Applies the value rules to free text: `"[redacted]"` if it holds a private key, an
/// `otpauth://` URI or an HTTP credential, otherwise the text with every URL password masked.
pub(super) fn redact_text(text: &str) -> String {
    if is_secret_text(text) {
        REDACTED.to_owned()
    } else {
        mask_url_passwords(text)
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
/// names a secret if any segment that is not an array index does.
fn label_names_secret(label: &str) -> bool {
    label
        .split(['/', '.'])
        .filter(|segment| {
            !segment.is_empty() && *segment != "-" && !segment.bytes().all(|b| b.is_ascii_digit())
        })
        .any(|segment| secrecy(segment) != Secrecy::Plain)
}

fn is_secret_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let start = lower.trim_start();
    (text.contains("-----BEGIN") && text.contains("PRIVATE KEY"))
        || SECRET_MARKERS.iter().any(|marker| text.contains(marker))
        || SECRET_MARKERS_ANY_CASE
            .iter()
            .any(|marker| lower.contains(marker))
        || CREDENTIAL_SCHEMES.iter().any(|scheme| {
            start
                .strip_prefix(scheme)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        })
}

/// Replaces the password of every `scheme://user:password@host` in `text`. The user ends at
/// the first `:` and holds no `/`, `?`, `#` or `@`; the password runs to the last `@` before
/// whitespace, so a password with `/`, `?`, `#` or `@` is masked whole (over-masking a URL that
/// has a port and an `@` later in its path).
fn mask_url_passwords(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme_end) = rest.find("://") {
        let (before, after) = rest.split_at(scheme_end + 3);
        masked.push_str(before);
        let token = &after[..after.find(char::is_whitespace).unwrap_or(after.len())];
        let userinfo = token
            .find(':')
            .filter(|&colon| !token[..colon].contains(['/', '?', '#', '@']))
            .and_then(|colon| {
                token
                    .rfind('@')
                    .filter(|&at| at > colon)
                    .map(|at| (colon, at))
            });
        rest = match userinfo {
            Some((colon, at)) => {
                masked.push_str(&after[..=colon]);
                masked.push_str(REDACTED);
                &after[at..]
            }
            None => after,
        };
    }
    masked.push_str(rest);
    masked
}
