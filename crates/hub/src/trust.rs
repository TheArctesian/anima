//! Trusting what a device presents (SEC-006, SEC-007, DEV-013): the SSH host key and the RDP
//! server certificate the operator accepted for each device, which every session connection is
//! pinned to; what connection tests saw and the operator has not accepted yet (`Sightings`);
//! and `acceptHostKey` and `acceptServerCertificate`, which store one of them.

mod accept;
mod accept_certificate;
mod certificate;
pub mod graphql;
mod model;
mod observations;
mod public_key;
mod sightings;
mod store;
#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;

use sqlx::PgPool;

use crate::{
    devices::{self, Device},
    events,
    ids::DeviceId,
};

pub(crate) use certificate::PresentedCertificate;
pub use model::{HostKey, ServerCertificate};
pub(crate) use observations::{Endpoint, Observation};
pub use public_key::{BadFingerprint, BadKey, Fingerprint, PublicKey};
pub use sightings::Sightings;

/// Why the operator cannot accept what they asked for (SEC-006, SEC-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Conflict {
    #[error(
        "The hub has not seen a host key for this device in the last 15 minutes. Test the \
         connection, then accept the key it shows."
    )]
    KeyNotSeen,
    #[error(
        "That fingerprint is not the key the hub saw for this device. Test the connection \
         again and compare the fingerprint."
    )]
    KeyDiffers,
    #[error(
        "The hub has not seen a server certificate for this device in the last 15 minutes. \
         Test the connection with RDP, then accept the certificate it shows."
    )]
    CertificateNotSeen,
    #[error(
        "That fingerprint is not the certificate the hub saw for this device. Test the \
         connection again and compare the fingerprint."
    )]
    CertificateDiffers,
}

/// Why a trust operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("device {0} not found")]
    NotFound(DeviceId),
    #[error(transparent)]
    Conflict(Conflict),
    /// The messages stay generic: the source errors may hold SQL text (API-010).
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("event store error")]
    Events(#[from] events::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The device, or `NotFound` for it.
async fn find_device(db: &PgPool, id: DeviceId) -> Result<Device, Error> {
    devices::get(db, id).await.map_err(|error| match error {
        devices::Error::NotFound(_) => Error::NotFound(id),
        other => Error::Devices(other),
    })
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

/// The accepted RDP server certificate of each listed device that has one, in one query
/// (API-005, SEC-007). An empty list returns an empty map without a query.
///
/// # Errors
/// `Database` when the query fails.
pub async fn server_certificates(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, ServerCertificate>, Error> {
    store::server_certificates(db, devices).await
}
