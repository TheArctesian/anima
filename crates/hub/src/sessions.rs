//! Remote sessions: the record of every session (RA-008) and the SSH host key accepted for
//! each device (SEC-006), which the gateway is pinned to (SEC-007).

pub mod graphql;
mod model;
mod page;
mod public_key;
mod store;
#[cfg(test)]
mod tests;
mod tunnel;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ids::DeviceId;

pub use model::{EndReason, HostKey, Protocol, Session, SessionEnd, UnknownValue};
pub use public_key::{BadFingerprint, BadKey, Fingerprint, PublicKey};

/// Why a sessions operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The host key accepted for the device, if any (SEC-006).
///
/// # Errors
/// `Database` when the query fails or the stored key does not parse.
pub async fn host_key(db: &PgPool, device: DeviceId) -> Result<Option<HostKey>, Error> {
    Ok(store::host_keys(db, &[device]).await?.remove(&device))
}

/// The accepted host key of each listed device that has one, in one query (API-005). An empty
/// list returns an empty map without a query.
///
/// # Errors
/// `Database` when the query fails or a stored key does not parse.
pub async fn host_keys(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, HostKey>, Error> {
    store::host_keys(db, devices).await
}

/// Ends, with `HUB_SHUTDOWN` at `now`, every session a previous hub process left open. The hub
/// calls it once at start, before it serves; returns how many rows it ended.
///
/// # Errors
/// `Database` when the update fails.
pub async fn close_stale(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    store::close_stale(db, now).await
}
