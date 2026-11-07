//! How a Guacamole tunnel ends: the shared end table (`end::cause`, K4 Section 5), and what
//! guacd's silence and a failed write to it mean (K4 Section 4).

use std::{io, time::Duration};

#[cfg(test)]
pub(super) use crate::sessions::end::Ending;
pub(super) use crate::sessions::end::{CLIENT_IDLE, CLOSE_WAIT, Cause, End};

/// The hub sends `nop` to the client when guacd has sent nothing for this long (K4 Section 4).
pub(super) const GATEWAY_SILENCE: Duration = Duration::from_secs(5);

/// The event message when the connection to guacd fails.
pub(super) const GATEWAY_LOST: &str =
    "The connection to the gateway failed, so the hub closed the session.";
/// The event message when a write to guacd stalls.
const GATEWAY_STOPPED_READING: &str =
    "The gateway stopped reading what the hub sent, so the hub closed the session.";

/// What one write to guacd means for the session: nothing when it went through, else a
/// gateway error. `None` is a write that stalled past its wait.
pub(super) fn gateway_written(written: Option<&io::Result<()>>) -> Result<(), End> {
    match written {
        Some(Ok(())) => Ok(()),
        Some(Err(_)) => Err(End::explained(Cause::GatewayError, GATEWAY_LOST)),
        None => Err(End::explained(Cause::GatewayError, GATEWAY_STOPPED_READING)),
    }
}
