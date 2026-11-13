//! Remote sessions: the record of every session (RA-008) and the single-use tickets that open
//! a tunnel (K4 §1, API-009). SSH sessions run on the terminal tunnel (`terminal`, K4 §8),
//! pinned to the host key `trust` holds (SEC-006, SEC-007); RDP and VNC sessions on the
//! Guacamole tunnel (`tunnel`). The connection test (`connection_test`) reads a host's key or
//! an RDP server's certificate into `trust::Sightings`, and `acceptHostKey` or
//! `acceptServerCertificate` in `trust` records it (DEV-013, SEC-006, SEC-007).
//! Wake-on-LAN (`wake`) runs on a site's wake relay over the hub's SSH link (`ssh::link`,
//! RA-031). The SSH client the terminal and the connection test use is in `ssh`.

mod admission;
mod connection_test;
mod end;
mod files;
pub mod graphql;
mod live;
mod model;
mod page;
mod quiet;
mod serving;
mod start;
mod store;
mod terminal;
#[cfg(test)]
mod tests;
mod tickets;
mod tunnel;
mod upgrade;
mod wake;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    api_errors::field_names,
    credentials,
    devices::{self, Device},
    events,
    ids::{DeviceId, SessionId},
    trust,
};

use model::Desktop;

pub use end::Actor;
pub use files::{FilesMutation, files_router};
pub use live::{EndCause, Live, Redeemed, Refused, Tunnel};
pub use model::{
    EndReason, FieldError, Protocol, Session, SessionChange, SessionEnd, UnknownValue,
};
pub use serving::Serving;
pub use start::{DEFAULT_DPI, DisplayRequest, StartRequest, Waking, start};
pub use terminal::terminal_router;
pub use tickets::{PendingTicket, Presented, Ticket};
pub use tunnel::tunnel_router;
pub use wake::{Error as WakeError, WakeSent, graphql::Mutation as WakeMutation, wake};

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
    #[error("RDP is turned off for this device. Turn it on in the device's RDP settings.")]
    RdpDisabled,
    #[error("VNC is turned off for this device. Turn it on in the device's VNC settings.")]
    VncDisabled,
    /// RDP checks the server certificate, and none is accepted yet (SEC-007).
    #[error(
        "No server certificate is accepted for this device. Test the connection and accept \
         its certificate."
    )]
    NoServerCertificate,
    /// The device has as many live sessions as its limit allows (RA-036).
    #[error("This device has reached its session limit.")]
    SessionLimit,
    /// The stored credential for the protocol was entered for another host than the device's
    /// host now; nothing was sent (SEC-011).
    #[error("{}", credential_moved(*.0))]
    CredentialMoved(Protocol),
}

/// The words of [`Conflict::CredentialMoved`] for `protocol`, which every refusal of a moved
/// credential uses: at start, in the tunnels and for files.
pub(in crate::sessions) const fn credential_moved(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Ssh => {
            "The device's address changed after its SSH credential was entered, so the hub \
             sent no credential. Enter the credential again in the device's SSH settings."
        }
        Protocol::Rdp => {
            "The device's address changed after its RDP password was entered, so the hub sent \
             no password. Enter the password again in the device's RDP settings."
        }
        Protocol::Vnc => {
            "The device's address changed after its VNC password was entered, so the hub sent \
             no password. Enter the password again in the device's VNC settings."
        }
    }
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
    #[error("trust store error")]
    Trust(#[from] trust::Error),
    #[error("no random bytes for a ticket")]
    Random(#[source] getrandom::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The device, or `NotFound` for it.
async fn find_device(db: &PgPool, id: DeviceId) -> Result<Device, Error> {
    devices::get(db, id)
        .await
        .map_err(|error| device_error(id, error))
}

/// A devices error reading device `id`, with a missing device as `NotFound` for it.
fn device_error(id: DeviceId, error: devices::Error) -> Error {
    match error {
        devices::Error::NotFound(_) => Error::NotFound(Missing::Device(id)),
        other => Error::Devices(other),
    }
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
