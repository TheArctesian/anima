//! Remote sessions: the record of every session (RA-008), the SSH host key accepted for
//! each device (SEC-006), which the gateway is pinned to (SEC-007), and the single-use
//! tickets that open the tunnel (K4 §1, API-009). The connection test reads a host's key
//! and `acceptHostKey` records it (DEV-013, D-5, D-6).

mod end;
pub mod graphql;
mod live;
mod model;
mod page;
mod probe;
mod public_key;
mod start;
mod store;
mod target;
#[cfg(test)]
mod tests;
mod tickets;
mod trust;
mod tunnel;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    credentials,
    devices::{self, Device},
    events,
    ids::{DeviceId, SessionId},
};

pub use end::Actor;
pub use live::{EndCause, Live, Redeemed, Refused, Tunnel};
pub use model::{EndReason, FieldError, HostKey, Protocol, Session, SessionEnd, UnknownValue};
pub use public_key::{BadFingerprint, BadKey, Fingerprint, PublicKey};
pub use start::{DEFAULT_DPI, DisplayRequest, StartRequest, start};
pub use tickets::{PendingTicket, Ticket};
pub use tunnel::tunnel_router;

/// What a sessions operation looked for and did not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Device(DeviceId),
    Session(SessionId),
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(id) => write!(f, "device {id}"),
            Self::Session(id) => write!(f, "session {id}"),
        }
    }
}

/// Why the device's state does not allow a session now (K4 §1, §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Conflict {
    #[error("SSH is turned off for this device. Turn it on in the device's SSH settings.")]
    SshDisabled,
    #[error("No host key is accepted for this device. Test the connection and accept its key.")]
    NoHostKey,
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
}

/// Why a sessions operation failed. No variant holds a ticket token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error(transparent)]
    Conflict(Conflict),
    /// Too many tickets are pending, or the hub is shutting down.
    #[error("no session can start now")]
    Unavailable,
    /// The messages stay generic: the source errors may hold SQL text (API-010).
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("event store error")]
    Events(#[from] events::Error),
    #[error("credential store error")]
    Credentials(#[source] credentials::Error),
    #[error("no random bytes for a ticket")]
    Random(#[source] getrandom::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The names of the fields in error, never their values.
fn field_names(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|error| error.field)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The device, or `NotFound` for it.
async fn find_device(db: &PgPool, id: DeviceId) -> Result<Device, Error> {
    devices::get(db, id).await.map_err(|error| match error {
        devices::Error::NotFound(_) => Error::NotFound(Missing::Device(id)),
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

/// Ends, with `HUB_SHUTDOWN` at `now`, every session a previous hub process left open, each
/// with its `session.end` audit entry and no event (D-13). The hub calls it once at start,
/// before it serves; returns how many rows it ended.
///
/// # Errors
/// `Database` or `Events` when the rows or audit entries cannot be written; then none is.
pub async fn close_stale(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    end::close_stale(db, now).await
}

/// Opens this hub process's sessions: ends what a previous process left open, as
/// [`close_stale`] does, and only then returns the `Live` that issues tickets, with how many
/// rows it ended (R-040, D-13). The hub calls it once, before it serves.
///
/// # Errors
/// As [`close_stale`]; then no `Live` is returned.
pub async fn open(db: &PgPool, now: DateTime<Utc>) -> Result<(Live, u64), Error> {
    let ended = end::close_stale(db, now).await?;
    Ok((Live::default(), ended))
}

/// Ends every session this hub holds with `HUB_SHUTDOWN` at `now`, then refuses new ones
/// (D-10). The hub calls it once on graceful shutdown, with the `Live` it serves, when the
/// shutdown signal arrives and before the runtime stops: tunnels are upgraded connections,
/// which the server's graceful drain does not end. It returns when every open
/// tunnel has written its end, or after `live`'s tunnel end wait (5 s by default); pending
/// tickets' rows end with their audit entry. What it misses, the next start's
/// [`close_stale`] ends.
///
/// # Errors
/// `Database` or `Events` when a pending ticket's row or audit entry cannot be written.
pub async fn shut_down(db: &PgPool, live: &Live, now: DateTime<Utc>) -> Result<(), Error> {
    end::shut_down(db, live, now).await
}
