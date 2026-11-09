//! Device inventory: sites, devices, tags and the SSH, RDP and VNC settings.

pub mod graphql;
mod loaders;
mod mac;
mod model;
mod page;
mod records;
mod remote;
mod rules;
mod store;
#[cfg(test)]
mod tests;
mod text;
mod warnings;

use std::num::NonZeroU8;

use sqlx::{PgConnection, PgExecutor, PgPool};

use crate::api_errors::field_names;
use crate::ids::{DeviceId, SiteId};

pub use loaders::{DevicesById, DevicesBySite, SitesById};
pub use mac::{MacAddress, MacAddressError};
pub use model::{Device, OperatingSystem, Site, SshAuth, SshSettings};
pub use remote::{
    MetricsSource, RdpSecurity, RdpSettings, ResizeMethod, TransferPolicy, VncSettings,
};
pub use rules::FieldError;

/// What a lookup could not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Device(DeviceId),
    Site(SiteId),
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(id) => write!(f, "device {id}"),
            Self::Site(id) => write!(f, "site {id}"),
        }
    }
}

/// Why a devices operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The device with this id, with its tags and SSH settings.
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn get(db: &PgPool, id: DeviceId) -> Result<Device, Error> {
    store::get(db, id).await
}

/// Every device, in byte order of the name.
///
/// # Errors
/// `Database` when the query fails.
pub async fn list(db: &PgPool) -> Result<Vec<Device>, Error> {
    store::all(db).await
}

/// Whether the device is in maintenance.
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn in_maintenance(db: &PgPool, id: DeviceId) -> Result<bool, Error> {
    store::in_maintenance(db, id).await
}

/// The device that sends Wake-on-LAN packets at the site, if one is set.
///
/// # Errors
/// `NotFound` when there is no such site.
pub async fn wake_relay(db: &PgPool, site: SiteId) -> Result<Option<Device>, Error> {
    store::wake_relay(db, site).await
}

/// Locks the device row (`FOR UPDATE`) until the caller's transaction ends, so no other
/// transaction changes or deletes the device meanwhile. A transaction that writes rows deleted
/// with the device takes this first, the order a device delete takes the rows in, so the two
/// cannot deadlock.
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn lock(conn: &mut PgConnection, id: DeviceId) -> Result<(), Error> {
    store::lock_device(conn, id).await
}

/// Sets the device's metrics source to `agent` in the caller's transaction, which stops SSH
/// collection for it, and returns the device's name. Only agent enrolment calls it (MET-011,
/// R-092 D-7).
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn use_agent_metrics(conn: &mut PgConnection, id: DeviceId) -> Result<String, Error> {
    store::set_agent_source(conn, id).await
}

/// The most live sessions the device allows, `None` for no limit (RA-036). Any executor, so
/// `sessions` reads it inside the transaction that starts a session.
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn session_limit<'e>(
    db: impl PgExecutor<'e>,
    id: DeviceId,
) -> Result<Option<NonZeroU8>, Error> {
    store::session_limit(db, id).await
}
