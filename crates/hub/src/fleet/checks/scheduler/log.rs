//! Warnings at the scheduler's I/O boundary. Error texts hold no SQL, URL, header or secret
//! (LOG-007).

use std::fmt::Display;

use crate::{
    fleet::{checks, events},
    ids::DeviceId,
};

/// Why a scheduler write that also writes an event failed; neither message holds SQL or a
/// secret (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum Fault {
    #[error(transparent)]
    Checks(#[from] checks::Error),
    #[error(transparent)]
    Events(#[from] events::Error),
}

impl From<sqlx::Error> for Fault {
    fn from(error: sqlx::Error) -> Self {
        Self::Checks(error.into())
    }
}

/// Logs a failed read or write, about one device when one is named.
pub(super) fn warned(device: Option<DeviceId>, what: &'static str, error: &dyn Display) {
    tracing::warn!(domain = "checks", device_id = ?device, %error, "{what}");
}

/// The value, with a failure logged.
pub(super) fn logged<T, E: Display>(
    device: Option<DeviceId>,
    what: &'static str,
    result: Result<T, E>,
) -> Option<T> {
    result.inspect_err(|error| warned(device, what, error)).ok()
}
