//! The connection test and accepting a host key (WP-10 T3; DEV-013, SEC-006, SEC-007, D-5,
//! D-6), against an in-process SSH server on the loopback interface. The hub's probe refuses
//! loopback, so these schemas share a `Live` built with the test-only loopback rule; the
//! refusal itself is tested with `Live::default()`.

mod accept;
mod connection_test;
mod deny_list;
mod leak;
mod server;

use anima_hub::{
    credentials::{Secret, SshSecret, store_ssh},
    ids::DeviceId,
    sessions::Live,
};
use async_graphql::{Request, Variables};
use russh::keys::{HashAlg, PrivateKey};
use serde_json::{Value, json};
use sqlx::PgPool;

use self::server::SshServer;
use super::{
    OPERATOR_ID, PVE_01, add_operator,
    harness::{TestSchema, schema_at, signed_in_until, vault},
    now,
};

pub const TEST: &str = "mutation($input: ConnectionTestInput!) { testConnection(input: $input) \
                        { reachable authenticated hostKey { algorithm fingerprintSha256 \
                        firstSeenAt } hostKeyChanged message } }";

pub const ACCEPT: &str = "mutation($id: ID!, $fingerprint: String!) { acceptHostKey(deviceId: \
                          $id, fingerprintSha256: $fingerprint) { algorithm fingerprintSha256 \
                          firstSeenAt } }";

/// A `Live` whose probes may reach the loopback test server.
pub fn loopback_live() -> Live {
    Live::default().reaching_loopback_for_tests()
}

/// The schema at `now()` sharing `live`.
pub fn schema(pool: &PgPool, live: &Live) -> TestSchema {
    schema_at(pool, now(), live)
}

/// `testConnection` with `input`, from the K6 operator.
pub fn test_request(input: &Value) -> Request {
    signed_in_until(TEST, 24).variables(Variables::from_json(json!({ "input": input })))
}

/// `acceptHostKey`, from the K6 operator.
pub fn accept_request(device: &str, fingerprint: &str) -> Request {
    let variables = json!({ "id": device, "fingerprint": fingerprint });
    signed_in_until(ACCEPT, 24).variables(Variables::from_json(variables))
}

/// The `testConnection` result, which must have no error.
pub async fn tested(schema: &TestSchema, input: &Value) -> Value {
    let response = schema.execute(test_request(input)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["testConnection"].clone()
}

/// Points PVE-01 at the test server on the loopback interface.
pub async fn point_at(pool: &PgPool, server: &SshServer) {
    sqlx::query("update devices.devices set host = '127.0.0.1' where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("update devices.ssh_settings set port = $2 where device_id = $1::text::uuid")
        .bind(PVE_01)
        .bind(i32::from(server.port))
        .execute(pool)
        .await
        .unwrap();
}

/// Stores `secret` as PVE-01's SSH credential, sealed with the schema's vault.
pub async fn store_credential(pool: &PgPool, secret: SshSecret) {
    let device: DeviceId = PVE_01.parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    store_ssh(&mut conn, &vault(), device, secret)
        .await
        .unwrap();
}

pub fn password(text: &str) -> SshSecret {
    SshSecret::Password(Secret::from(text.to_owned()))
}

/// OpenSSH's own fingerprint of the key, computed by `ssh-key`, not by the hub.
pub fn fingerprint(key: &PrivateKey) -> String {
    key.public_key().fingerprint(HashAlg::Sha256).to_string()
}

/// Stores `key` as PVE-01's accepted host key, as an operator accepted it earlier.
pub async fn accept_directly(pool: &PgPool, key: &PrivateKey) {
    add_operator(pool).await;
    let openssh = key.public_key().to_openssh().unwrap();
    let mut parts = openssh.split(' ');
    let (algorithm, base64) = (parts.next().unwrap(), parts.next().unwrap());
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4, $5, $5, $6::text::uuid)",
    )
    .bind(PVE_01)
    .bind(algorithm)
    .bind(fingerprint(key))
    .bind(base64)
    .bind(now())
    .bind(OPERATOR_ID)
    .execute(pool)
    .await
    .unwrap();
}
