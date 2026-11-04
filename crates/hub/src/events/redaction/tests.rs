use serde_json::{Value, json};

use super::redact;

const PEM: &str =
    "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk=\n-----END OPENSSH PRIVATE KEY-----";

fn kept(change: &Value) {
    assert_eq!(&redact(change.clone()), change);
}

/// Asserts that the value of every listed key is redacted.
fn hidden(keys: &[&str]) {
    let change: serde_json::Map<String, Value> = keys
        .iter()
        .map(|key| ((*key).to_owned(), json!("s3cr3t")))
        .collect();
    let expected: serde_json::Map<String, Value> = keys
        .iter()
        .map(|key| ((*key).to_owned(), json!("[redacted]")))
        .collect();
    assert_eq!(redact(Value::Object(change)), Value::Object(expected));
}

#[test]
fn log_007_redaction_replaces_secret_keys_at_any_depth_and_inside_arrays() {
    let change = json!({
        "Password": "a",
        "name": "lab-router",
        "ssh": { "PRIVATE_KEY": "b", "privateKey": { "nested": "c" }, "port": 22 },
        "steps": [{ "totpCode": "1" }, [{ "Token": "e" }]],
    });

    assert_eq!(
        redact(change),
        json!({
            "Password": "[redacted]",
            "name": "lab-router",
            "ssh": { "PRIVATE_KEY": "[redacted]", "privateKey": "[redacted]", "port": 22 },
            "steps": [{ "totpCode": "[redacted]" }, [{ "Token": "[redacted]" }]],
        })
    );
}

#[test]
fn log_007_redaction_covers_every_secret_word_and_spelling() {
    hidden(&[
        "password",
        "passwd",
        "passphrase",
        "apiSecret",
        "newPassword",
        "credential",
    ]);
    hidden(&[
        "Authorization",
        "Cookie",
        "bearerValue",
        "pass",
        "PWD",
        "sshPass",
        "pin",
    ]);
    hidden(&[
        "otp",
        "OTP",
        "otpCode",
        "totpSecret",
        "otpauthUri",
        "auth",
        "sshKeyPem",
    ]);
    hidden(&[
        "key",
        "apiKey",
        "APIKey",
        "ssh_key",
        "apiKeys",
        "keyMaterial",
        "signingKeyMaterial",
    ]);
    hidden(&[
        "apikey",
        "sshkey",
        "accesskey",
        "signingkey",
        "encryptionkey",
        "sshkeys",
    ]);
}

#[test]
fn log_007_redaction_sees_through_unicode_tricks() {
    hidden(&[
        "pass\u{200b}word",
        "to\u{200d}ken",
        "\u{ff30}\u{ff41}\u{ff53}\u{ff53}\u{ff57}\u{ff4f}\u{ff52}\u{ff44}",
        "p\u{e4}ssword",
        "pa\u{301}ssword",
    ]);
}

#[test]
fn log_007_redaction_hides_everything_beside_a_label_that_names_a_secret() {
    let change = json!([
        { "op": "replace", "path": "/ssh/password", "value": "a" },
        { "field": "password", "before": "b", "after": "c", "oldValue": "d", "newValue": "e" },
        { "key": "password", "value": "f" },
        { "setting": "privateKey", "value": { "pem": "g" }, "enabled": true },
        { "field": "ssh.passphrase", "change": { "from": "h", "to": "i" } },
        { "property": "API_TOKEN", "value": 1 },
        { "op": "add", "path": "/secrets/0", "value": "j" },
    ]);

    assert_eq!(
        redact(change),
        json!([
            { "op": "replace", "path": "/ssh/password", "value": "[redacted]" },
            {
                "field": "password",
                "before": "[redacted]",
                "after": "[redacted]",
                "oldValue": "[redacted]",
                "newValue": "[redacted]",
            },
            { "key": "password", "value": "[redacted]" },
            { "setting": "privateKey", "value": "[redacted]", "enabled": true },
            { "field": "ssh.passphrase", "change": "[redacted]" },
            { "property": "API_TOKEN", "value": "[redacted]" },
            { "op": "add", "path": "/secrets/0", "value": "[redacted]" },
        ])
    );
}

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
fn log_007_redaction_keeps_values_without_secrets() {
    kept(&json!(["password", { "username": "admin", "port": 22, "tags": ["token"] }, null]));
    kept(&json!({ "keyboard": "us", "monkey": 1, "keyId": "k-1", "deviceName": "x" }));
    kept(&json!({ "notPaused": true, "footprint": "small", "hotplug": false }));
    kept(
        &json!({ "hostKey": "SHA256:abc", "hostKeys": ["SHA256:d"], "publicKey": "ssh-ed25519 AAAA", "nodeKey": "nodekey:1" }),
    );
    kept(&json!({ "field": "name", "from": "lab-router", "to": "core-router" }));
    kept(&json!({ "op": "replace", "path": "/ssh/port", "value": 2222 }));
    kept(&json!({ "op": "replace", "path": "/passwordPolicy/minLength", "value": 12 }));
    kept(&json!({ "name": "secret-lab", "value": "kept", "from": "a" }));
}

#[test]
fn log_007_redaction_never_hides_a_boolean() {
    kept(&json!({ "hasPassword": true, "credentialStored": false, "password": true }));
}

#[test]
fn log_007_redaction_over_matches_timestamps_named_for_secrets_deliberately() {
    let change = json!({ "tokenExpiresAt": "2026-11-03T00:00:00Z", "passwordChangedAt": "x" });
    assert_eq!(
        redact(change),
        json!({ "tokenExpiresAt": "[redacted]", "passwordChangedAt": "[redacted]" })
    );
}
