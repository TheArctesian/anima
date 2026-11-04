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
        "passcode",
    ]);
    hidden(&[
        "authKey",
        "privkey",
        "jwt",
        "recoveryCode",
        "backupCodes",
        "mfaCode",
        "pw",
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
        { "op": "replace", "path": "/password/value", "value": "k" },
        { "op": "replace", "path": "/credentials/0/value", "value": "l" },
        { "name": "Authorization", "value": "m" },
        { "name": "X-Api-Key", "value": "n" },
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
            { "op": "replace", "path": "/password/value", "value": "[redacted]" },
            { "op": "replace", "path": "/credentials/0/value", "value": "[redacted]" },
            { "name": "Authorization", "value": "[redacted]" },
            { "name": "X-Api-Key", "value": "[redacted]" },
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
fn log_007_redaction_hides_http_credentials_and_putty_keys_under_any_key() {
    let change = json!([
        "Bearer eyJhbGciOi",
        "  basic YWRtaW46aHVudGVyMg==",
        "curl -H 'Authorization: Bearer x' https://hub",
        "see OTPAUTH://totp/x?secret=Y",
        "PuTTY-User-Key-File-3: ssh-ed25519",
    ]);

    assert_eq!(redact(change), Value::Array(vec![json!("[redacted]"); 5]));
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
    });

    assert_eq!(
        redact(change),
        json!({
            "url": "postgres://anima:[redacted]@db:5432/anima",
            "mirrors": "https://a:[redacted]@one.example/x and ftp://c:[redacted]@two.example",
            "plain": "https://user@host.example/path",
            "other": "https://host.example/a:b@c",
        })
    );
}

#[test]
fn log_007_redaction_keeps_only_metadata_inside_a_credential_holder() {
    let change = json!({
        "credential": { "kind": "password", "username": "admin", "id": "c1", "password": "x" },
        "sshCredentials": [{ "username": "root", "privateKey": "y", "type": "ed25519" }],
        "credentialId": "c1",
        "label": { "credential": "z" },
    });

    assert_eq!(
        redact(change),
        json!({
            "credential": { "kind": "password", "username": "admin", "id": "c1", "password": "[redacted]" },
            "sshCredentials": [{ "username": "root", "privateKey": "[redacted]", "type": "ed25519" }],
            "credentialId": "c1",
            "label": { "credential": "[redacted]" },
        })
    );
}

#[test]
fn log_007_redaction_hides_every_other_scalar_inside_a_credential_holder() {
    let change = json!([
        { "credential": { "kind": "password", "value": "hunter2" } },
        { "credential": { "username": "u", "data": "hunter2" } },
        { "credentials": ["hunter2"] },
        { "credential": { "kind": { "value": "hunter2" }, "id": ["hunter2"], "ssh": { "port": 22 } } },
        { "credential": { "created_at": "2026-11-04", "rotatedAt": "x", "fingerprint": "SHA256:a" } },
        { "credential": { "Stored": true, "enabled": false, "note": null, "userName": PEM } },
        { "sshCredential": [["hunter2", 1234, null, true]] },
        { "credential": { "passwordChangedAt": "x", "privateKey": { "kind": "ed25519" } } },
    ]);

    assert_eq!(
        redact(change),
        json!([
            { "credential": { "kind": "password", "value": "[redacted]" } },
            { "credential": { "username": "u", "data": "[redacted]" } },
            { "credentials": ["[redacted]"] },
            {
                "credential": {
                    "kind": { "value": "[redacted]" },
                    "id": ["[redacted]"],
                    "ssh": { "port": "[redacted]" },
                },
            },
            { "credential": { "created_at": "2026-11-04", "rotatedAt": "x", "fingerprint": "SHA256:a" } },
            { "credential": { "Stored": true, "enabled": false, "note": "[redacted]", "userName": "[redacted]" } },
            { "sshCredential": [["[redacted]", "[redacted]", "[redacted]", true]] },
            { "credential": { "passwordChangedAt": "[redacted]", "privateKey": "[redacted]" } },
        ])
    );
}

#[test]
fn log_007_redaction_hides_private_keys_named_like_public_keys() {
    hidden(&[
        "privateHostKey",
        "privateNodeKey",
        "hostKeyPrivate",
        "privHostKey",
        "private_public_key",
        "privateHostKeys",
    ]);
    kept(&json!({ "privateIp": "10.0.0.1", "privateNetwork": "lab", "privilegeLevel": 2 }));
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
    kept(&json!({ "name": "secret-lab", "to": "kept", "from": "a" }));
    kept(&json!({ "name": "Accept", "value": "application/json" }));
}

#[test]
fn log_007_redaction_keeps_public_keys_and_certificate_pins_as_audit_evidence() {
    kept(&json!({
        "newHostKey": "SHA256:abc",
        "sshHostKey": "SHA256:def",
        "authorizedKeys": ["ssh-ed25519 AAAA"],
        "authorized_key": "ssh-ed25519 BBBB",
        "certPin": "sha256/abc",
        "tlsPinSha256": "def",
        "spkiPins": ["ghi"],
    }));
}

#[test]
fn log_007_redaction_never_hides_a_boolean() {
    kept(&json!({ "hasPassword": true, "credentialStored": false, "password": true }));
}

#[test]
fn log_007_redaction_over_matches_timestamps_named_for_secrets_deliberately() {
    let change = json!([
        { "tokenExpiresAt": "2026-11-03T00:00:00Z", "passwordChangedAt": "x" },
        { "op": "replace", "path": "/passwordPolicy/minLength", "value": 12 },
    ]);
    assert_eq!(
        redact(change),
        json!([
            { "tokenExpiresAt": "[redacted]", "passwordChangedAt": "[redacted]" },
            { "op": "replace", "path": "/passwordPolicy/minLength", "value": "[redacted]" },
        ])
    );
}
