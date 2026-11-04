//! `Session` and `HostKey` as the GraphQL types K1 describes. Neither exposes the client
//! address, the operator id or the key blob.

use async_graphql::{ID, Object};
use chrono::{DateTime, Utc};

use crate::sessions::{EndReason, HostKey, Protocol, Session};

#[Object]
impl Session {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn device_id(&self) -> Option<ID> {
        self.device.map(|device| ID(device.to_string()))
    }

    /// The device's name when the session started; kept after the device is deleted.
    async fn device_name(&self) -> &str {
        &self.device_name
    }

    async fn protocol(&self) -> Protocol {
        self.protocol
    }

    async fn operator_name(&self) -> &str {
        &self.operator_name
    }

    async fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    async fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.end.map(|end| end.at)
    }

    async fn end_reason(&self) -> Option<EndReason> {
        self.end.map(|end| end.reason)
    }
}

#[Object]
impl HostKey {
    async fn algorithm(&self) -> &str {
        self.public_key.algorithm()
    }

    /// OpenSSH's `SHA256:` fingerprint of the key.
    async fn fingerprint_sha256(&self) -> String {
        self.public_key.fingerprint().to_string()
    }

    async fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
    }
}
