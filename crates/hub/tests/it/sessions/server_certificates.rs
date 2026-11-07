//! The RDP and VNC connection test and accepting an RDP server certificate (DEV-013, SEC-007,
//! LOG-005), against in-process servers on the loopback interface. As for host keys, these
//! schemas share a `Live` with the test-only loopback rule; the refusal itself is tested with
//! `Live::default()`.

mod accept;
mod connection_test;
mod matched;
mod servers;

use async_graphql::{Request, Response, Variables};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{PVE_01, harness::signed_in_until};

pub const TEST: &str = "mutation($input: ConnectionTestInput!) { testConnection(input: $input) \
                        { reachable authenticated hostKey { fingerprintSha256 } hostKeyChanged \
                        serverCertificate { fingerprintSha256 subject notAfter firstSeenAt } \
                        certificateChanged certificateMatched message } }";

pub const ACCEPT: &str = "mutation($id: ID!, $fingerprint: String!) { \
                          acceptServerCertificate(deviceId: $id, fingerprintSha256: \
                          $fingerprint) { fingerprintSha256 subject notAfter firstSeenAt } }";

/// `testConnection` with `input`, from the K6 operator.
pub fn test_request(input: &Value) -> Request {
    signed_in_until(TEST, 24).variables(Variables::from_json(json!({ "input": input })))
}

/// `acceptServerCertificate`, from the K6 operator.
pub fn accept_request(device: &str, fingerprint: &str) -> Request {
    let variables = json!({ "id": device, "fingerprint": fingerprint });
    signed_in_until(ACCEPT, 24).variables(Variables::from_json(variables))
}

/// The `testConnection` result of a response that must have no error.
pub fn result(response: Response) -> Value {
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["testConnection"].clone()
}

/// A `DateTime` field of a response.
pub fn instant(field: &Value) -> DateTime<Utc> {
    field.as_str().unwrap().parse().unwrap()
}

/// Points PVE-01 at 127.0.0.1 with its RDP and VNC ports at `port`, and RDP `security`
/// (lower-case, as stored) and `ignore_certificate`.
pub async fn point_at(pool: &PgPool, port: u16, security: &str, ignore_certificate: bool) {
    sqlx::query("update devices.devices set host = '127.0.0.1' where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "update devices.rdp_settings set port = $2, security = $3, ignore_certificate = $4 \
         where device_id = $1::text::uuid",
    )
    .bind(PVE_01)
    .bind(i32::from(port))
    .bind(security)
    .bind(ignore_certificate)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("update devices.vnc_settings set port = $2 where device_id = $1::text::uuid")
        .bind(PVE_01)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}
