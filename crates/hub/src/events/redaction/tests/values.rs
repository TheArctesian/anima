//! The value rules: strings that are secrets under any key (LOG-007, D-3).

use serde_json::{Value, json};

use super::{PEM, kept};
use crate::events::redaction::redact;

#[test]
fn log_007_redaction_hides_private_keys_and_otpauth_uris_under_any_key() {
    let change = json!({
        "notes": PEM,
        "lines": ["fine", PEM],
        "enrolment": "otpauth://totp/anima:operator?secret=JBSWY3DPEHPK3PXP",
    });

    assert_eq!(
        redact(change),
        json!({ "notes": "[redacted]", "lines": ["fine", "[redacted]"], "enrolment": "[redacted]" })
    );
    assert_eq!(redact(json!(PEM)), json!("[redacted]"));
}

#[test]
fn log_007_redaction_hides_http_credentials_and_putty_keys_under_any_key() {
    let change = json!([
        "Bearer eyJhbGciOi",
        "Bearer\teyJhbGciOi",
        "  basic YWRtaW46aHVudGVyMg==",
        "curl -H 'Authorization: Bearer x' https://hub",
        "see OTPAUTH://totp/x?secret=Y",
        "PuTTY-User-Key-File-3: ssh-ed25519",
    ]);

    assert_eq!(redact(change), Value::Array(vec![json!("[redacted]"); 6]));
    kept(&json!([
        "not a Bearer token",
        "basically fine",
        "authorization pending"
    ]));
}

#[test]
fn log_007_redaction_masks_the_password_in_a_url() {
    let change = json!({
        "url": "postgres://anima:hunter2@db:5432/anima",
        "mirrors": "https://a:b@one.example/x and ftp://c:d:e@two.example",
        "plain": "https://user@host.example/path",
        "other": "https://host.example/a:b@c",
        "slash": "https://u:pa/ss@host.example/x",
        "query": "postgres://u:p?w#d@db/anima ok",
        "nested": "https://host.example/?next=https://v:q@k.example",
    });

    assert_eq!(
        redact(change),
        json!({
            "url": "postgres://anima:[redacted]@db:5432/anima",
            "mirrors": "https://a:[redacted]@one.example/x and ftp://c:[redacted]@two.example",
            "plain": "https://user@host.example/path",
            "other": "https://host.example/a:b@c",
            "slash": "https://u:[redacted]@host.example/x",
            "query": "postgres://u:[redacted]@db/anima ok",
            "nested": "https://host.example/?next=https://v:[redacted]@k.example",
        })
    );
}
