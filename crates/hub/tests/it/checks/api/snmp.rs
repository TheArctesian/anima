//! SNMP checks through the API: the read shows no secret, and neither does any error, audit
//! row, event, broadcast or log line (D-5, SEC-003, SEC-005, LOG-007, K1 `SnmpConfig`).

mod errors;
mod reads;

use serde_json::{Value, json};

pub const COMMUNITY: &str = "hunter2-snmp-community";
pub const AUTH: &str = "hunter2-snmp-auth";
pub const PRIVACY: &str = "hunter2-snmp-privacy";
pub const SECRETS: [&str; 3] = [COMMUNITY, AUTH, PRIVACY];

pub const READ: &str = "query($id: ID!) { check(id: $id) { type config { __typename \
    ... on SnmpConfig { port version oid expected username timeoutMs } } } }";

/// A v3 configuration input with both passwords.
pub fn v3(username: &str) -> Value {
    json!({ "snmp": {
        "version": "V3",
        "oid": "1.3.6.1.2.1.1.5.0",
        "username": username,
        "authPassword": AUTH,
        "privacyPassword": PRIVACY,
    } })
}
