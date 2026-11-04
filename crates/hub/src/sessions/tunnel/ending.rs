//! How a tunnel ends: the one table from cause to close code, end reason and event severity
//! (K4 Section 5, R-032, WP-10 D-9).

use std::time::Duration;

use crate::events::Severity;
use crate::sessions::EndReason;

/// The hub sends `nop` to the client when guacd has sent nothing for this long (K4 Section 4).
pub(super) const GATEWAY_SILENCE: Duration = Duration::from_secs(5);
/// The hub ends the session with `TIMEOUT` when the client has sent nothing for this long,
/// not even a ping (K4 Section 4).
pub(super) const CLIENT_IDLE: Duration = Duration::from_secs(15);

/// Why a live tunnel ends. The first cause wins; the tunnel task records it once (D-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EndCause {
    /// The client closed the WebSocket or sent `disconnect`, or `endSession` ran.
    OperatorClosed,
    /// guacd sent `disconnect`, or its connection closed (EOF).
    RemoteClosed,
    /// guacd sent `error` with a status other than 0x0301 or 0x0303, or bytes that do not
    /// decode.
    GatewayError,
    /// guacd sent `error` with 0x0301 (unauthorized) or 0x0303 (forbidden).
    AuthFailed,
    /// The device's host key is not the accepted one (SEC-007).
    HostKeyMismatch,
    /// The client sent nothing for [`CLIENT_IDLE`].
    Timeout,
    /// The hub is shutting down.
    HubShutdown,
    /// The client sent bytes that do not decode, an opcode outside the allow-list, or an
    /// instruction whose shape guacd cannot handle (GW-011).
    InvalidInstruction,
}

/// What the tunnel task does at the end: the WebSocket close code for the client, the
/// session's `end_reason` and the severity of the `session.end` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Ending {
    pub(super) close_code: u16,
    pub(super) reason: EndReason,
    pub(super) severity: Severity,
}

impl EndCause {
    /// The K4 Section 5 row for this cause; `InvalidInstruction` ends `PROTOCOL_ERROR` (R-034).
    pub(super) const fn ending(self) -> Ending {
        let (close_code, reason, severity) = match self {
            Self::OperatorClosed => (1000, EndReason::Operator, Severity::Info),
            Self::RemoteClosed => (1000, EndReason::RemoteClosed, Severity::Info),
            Self::GatewayError => (1011, EndReason::GatewayError, Severity::Warning),
            Self::AuthFailed => (1011, EndReason::AuthFailed, Severity::Warning),
            Self::HostKeyMismatch => (4001, EndReason::HostKeyMismatch, Severity::Error),
            Self::Timeout => (4002, EndReason::Timeout, Severity::Info),
            Self::HubShutdown => (1001, EndReason::HubShutdown, Severity::Info),
            Self::InvalidInstruction => (1002, EndReason::ProtocolError, Severity::Warning),
        };
        Ending {
            close_code,
            reason,
            severity,
        }
    }
}
