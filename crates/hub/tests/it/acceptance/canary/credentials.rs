//! A device's SSH password, private key and passphrase, RDP and VNC passwords, a credential the
//! hub cannot read, and a connection test to a refused host that carries a password (DEV-011, DEV-012, DEV-013,
//! SEC-003, LOG-007).

use serde_json::{Value, json};

use super::{
    Secrets,
    edge::{WholeHub, code},
};

/// `pve-01` in the `devices` fixture.
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

const SSH: &str = "mutation($id: ID!, $c: SshCredentialInput!) { \
    setSshCredential(deviceId: $id, credential: $c) { credentialStored } }";
const RDP: &str = "mutation($id: ID!, $c: PasswordCredentialInput!) { \
    setRdpCredential(deviceId: $id, credential: $c) { credentialStored } }";
const VNC: &str = "mutation($id: ID!, $c: PasswordCredentialInput!) { \
    setVncCredential(deviceId: $id, credential: $c) { credentialStored } }";
const TEST: &str = "mutation($input: ConnectionTestInput!) { \
    testConnection(input: $input) { reachable authenticated } }";

/// A private key whose body is `body`: the hub checks only the armour.
fn private_key(body: &str, passphrase: Option<&str>) -> Value {
    let key =
        format!("-----BEGIN OPENSSH PRIVATE KEY-----\n{body}\n-----END OPENSSH PRIVATE KEY-----\n");
    json!({ "privateKey": { "key": key, "passphrase": passphrase } })
}

/// Stores each credential kind for `pve-01` with the operator's `token`, then sends what the
/// hub refuses or tests.
pub async fn set(hub: &WholeHub, token: &str, secrets: &mut Secrets) {
    let key = private_key(
        &secrets.plant("ssh-key"),
        Some(&secrets.plant("ssh-passphrase")),
    );
    let kinds = [
        (
            SSH,
            "setSshCredential",
            json!({ "password": secrets.plant("ssh-password") }),
        ),
        (SSH, "setSshCredential", key),
        (
            RDP,
            "setRdpCredential",
            json!({ "password": secrets.plant("rdp-password") }),
        ),
        (
            VNC,
            "setVncCredential",
            json!({ "password": secrets.plant("vnc-password") }),
        ),
    ];
    for (query, field, credential) in kinds {
        let stored = hub
            .data(token, query, json!({ "id": PVE_01, "c": credential }))
            .await;
        assert_eq!(stored[field]["credentialStored"], true, "{field}");
    }
    let unreadable = json!({ "id": PVE_01, "c": secrets.plant("unreadable-ssh") });
    let refused = hub.as_bearer(token, SSH, unreadable).await;
    assert_eq!(
        code(&refused),
        "VALIDATION",
        "a bare string is no credential"
    );

    // The hub refuses a loopback host before it connects (R-032): the refusal holds the
    // typed password too.
    let input = json!({ "host": "127.0.0.1",
        "ssh": { "enabled": true, "username": "operator", "auth": "PASSWORD" },
        "credential": { "password": secrets.plant("tested-ssh-password") } });
    let refused = hub.as_bearer(token, TEST, json!({ "input": input })).await;
    assert_eq!(code(&refused), "VALIDATION", "a loopback host is refused");
}
