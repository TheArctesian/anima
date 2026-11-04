//! Credential holders and `auth` keep only metadata and method names (LOG-007, D-3).

use serde_json::json;

use super::PEM;
use crate::events::redaction::redact;

#[test]
fn log_007_redaction_keeps_only_metadata_inside_a_credential_holder() {
    let change = json!({
        "credential": { "kind": "password", "username": "admin", "id": "c1", "password": "x" },
        "sshCredentials": [{ "username": "root", "privateKey": "y", "type": "ed25519" }],
        "credentialId": "c1",
        "label": { "credential": "z" },
        "creds": "z",
        "sshCreds": [{ "username": "u", "value": "z" }],
    });

    assert_eq!(
        redact(change),
        json!({
            "credential": { "kind": "password", "username": "admin", "id": "c1", "password": "[redacted]" },
            "sshCredentials": [{ "username": "root", "privateKey": "[redacted]", "type": "ed25519" }],
            "credentialId": "c1",
            "label": { "credential": "[redacted]" },
            "creds": "[redacted]",
            "sshCreds": [{ "username": "u", "value": "[redacted]" }],
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
        { "credential": { "expiresAt": "x", "lastUsedAt": "x", "updatedAt": "x", "storedAt": "x" } },
        { "credential": { "valueAt": "hunter2", "oauthAT": "hunter2", "at": "hunter2" } },
        { "credential": { "stored": "hunter2", "note": "2026-11-04T10:00:00Z" } },
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
            { "credential": { "expiresAt": "x", "lastUsedAt": "x", "updatedAt": "x", "storedAt": "x" } },
            { "credential": { "valueAt": "[redacted]", "oauthAT": "[redacted]", "at": "[redacted]" } },
            { "credential": { "stored": "[redacted]", "note": "2026-11-04T10:00:00Z" } },
            { "credential": { "Stored": true, "enabled": false, "note": "[redacted]", "userName": "[redacted]" } },
            { "sshCredential": [["[redacted]", "[redacted]", "[redacted]", true]] },
            { "credential": { "passwordChangedAt": "[redacted]", "privateKey": "[redacted]" } },
        ])
    );
}

#[test]
fn log_007_redaction_keeps_an_auth_method_and_hides_any_other_auth_value() {
    let change = json!([
        { "auth": "dXNlcjpodW50ZXIy" },
        { "basicAuth": "dXNlcjpodW50ZXIy", "proxyAuth": "u:hunter2" },
        { "auth": 1234 },
        { "auth": { "user": "u", "pass": "x", "kind": "basic" } },
        { "auth": "password" },
        { "auth": "private_key" },
        { "Auth": "PublicKey" },
        { "auth": "none", "ssh": { "auth": "key" } },
        { "auth": "token" },
        { "auth": "TOTP" },
        { "auth": true },
    ]);

    assert_eq!(
        redact(change),
        json!([
            { "auth": "[redacted]" },
            { "basicAuth": "[redacted]", "proxyAuth": "[redacted]" },
            { "auth": "[redacted]" },
            { "auth": { "user": "[redacted]", "pass": "[redacted]", "kind": "basic" } },
            { "auth": "password" },
            { "auth": "private_key" },
            { "Auth": "PublicKey" },
            { "auth": "none", "ssh": { "auth": "key" } },
            { "auth": "token" },
            { "auth": "TOTP" },
            { "auth": true },
        ])
    );
}
