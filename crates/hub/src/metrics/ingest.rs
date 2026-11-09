//! The Anima agent's side of the hub: enrolment codes, agent tokens, the `/agent/v1/` routes
//! and the stale mark of an agent that stopped pushing (K8, MET-011 to MET-018, R-092 D-1).
//!
//! A code and a token are kept only as the SHA-256 of their random bytes, and each belongs to
//! one device. Neither reaches a log, an event, an error or any response other than the one
//! that hands it out (LOG-007). The core takes the time and the new secret as parameters; the
//! OS random source is read only by the two edges, the routes and the GraphQL mutations.

mod credential;
mod enrolment;
mod push;
mod refusals;
mod renewal;
mod routes;
mod stale;
mod store;
#[cfg(test)]
mod tests;
mod throttle;

use chrono::TimeDelta;
use zeroize::Zeroizing;

pub use enrolment::AgentEnrolment;
pub(super) use enrolment::{make_code, revoke};
pub use routes::agent_router;
pub use stale::spawn_stale_watch;
pub(super) use stale::without_fresh_agent;

use crate::{devices, events};

/// How long an enrolment code works (MET-012).
const CODE_LIFETIME: TimeDelta = TimeDelta::minutes(15);

/// How long an agent token works, from enrolment or renewal (MET-013).
const TOKEN_LIFETIME: TimeDelta = TimeDelta::days(365);

/// Why an agent operation failed. Messages are fixed text: they hold no code, token, body or
/// SQL (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("the event log cannot be written")]
    Events(#[from] events::Error),
    #[error("the OS random source failed")]
    Random(#[source] getrandom::Error),
    /// The device was deleted while its agent enrolled.
    #[error("the device does not exist")]
    NoDevice,
    #[error("the device cannot be read")]
    Devices(#[source] devices::Error),
    #[error("the metrics cannot be stored")]
    Metrics(#[from] super::Error),
}

impl From<devices::Error> for Error {
    fn from(error: devices::Error) -> Self {
        match error {
            devices::Error::NotFound(_) => Self::NoDevice,
            other => Self::Devices(other),
        }
    }
}

/// `N` bytes from the OS random source, wiped on drop (R-092 D-1).
///
/// # Errors
/// `Random` when the source fails.
pub(super) fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>, Error> {
    let mut bytes = Zeroizing::new([0_u8; N]);
    getrandom::fill(bytes.as_mut_slice()).map_err(Error::Random)?;
    Ok(bytes)
}
