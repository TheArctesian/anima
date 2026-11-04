//! Warnings at the scheduler's I/O boundary. Error texts hold no SQL, URL, header or secret
//! (LOG-007).

use std::fmt::Display;

use crate::ids::DeviceId;

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
