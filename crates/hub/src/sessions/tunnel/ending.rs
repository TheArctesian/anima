//! How a tunnel ends: the one table from cause to close code, end reason and event severity
//! (K4 Section 5, R-032, WP-10 D-9).

use std::time::Duration;

use crate::events::Severity;
use crate::sessions::{Actor, EndCause, EndReason};

/// The hub sends `nop` to the client when guacd has sent nothing for this long (K4 Section 4).
pub(super) const GATEWAY_SILENCE: Duration = Duration::from_secs(5);
/// The hub ends the session with `TIMEOUT` when the client has sent nothing for this long,
/// not even a ping (K4 Section 4).
pub(super) const CLIENT_IDLE: Duration = Duration::from_secs(15);
/// The longest the hub waits for each peer while it closes: telling guacd to disconnect and
/// sending the client its close frame run side by side, so a stalled peer delays the end's
/// write by at most this, well inside `endSession`'s wait for the tunnel (D-10).
pub(super) const CLOSE_WAIT: Duration = Duration::from_secs(2);

/// Why a live tunnel ends. The first cause wins; the tunnel task records it once (D-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cause {
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
    /// The client sent nothing for [`CLIENT_IDLE`], or stopped reading what the hub sends.
    Timeout,
    /// The hub is shutting down.
    HubShutdown,
    /// The client sent bytes that do not decode, an opcode outside the allow-list, an
    /// instruction whose shape guacd cannot handle, a binary message, a message over the size
    /// cap or text that is not UTF-8 (GW-011, R-039).
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

impl Cause {
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

    /// The event message for an end with this cause. Fixed text: it never holds guacd's
    /// words, an address, a value or session content (LOG-007, RA-008).
    pub(super) const fn message(self) -> &'static str {
        match self {
            Self::OperatorClosed => "The operator closed the SSH session.",
            Self::RemoteClosed => "The device or the gateway closed the SSH session.",
            Self::GatewayError => "The gateway reported an error and closed the SSH session.",
            Self::AuthFailed => "The device refused the SSH sign-in.",
            Self::HostKeyMismatch => {
                "The device presented an SSH host key that is not the accepted one, so the hub \
                 closed the session before signing in. Test the connection and check the new \
                 key before accepting it."
            }
            Self::Timeout => {
                "The client sent nothing for too long, so the hub closed the SSH session."
            }
            Self::HubShutdown => "The hub shut down and closed the SSH session.",
            Self::InvalidInstruction => {
                "The client sent an instruction the hub does not accept, so the hub closed the \
                 SSH session."
            }
        }
    }
}

impl From<EndReason> for Cause {
    /// The cause behind a reason signalled from outside the tunnel (`endSession`, shutdown).
    fn from(reason: EndReason) -> Self {
        match reason {
            EndReason::Operator => Self::OperatorClosed,
            EndReason::RemoteClosed => Self::RemoteClosed,
            EndReason::GatewayError => Self::GatewayError,
            EndReason::AuthFailed => Self::AuthFailed,
            EndReason::HostKeyMismatch => Self::HostKeyMismatch,
            EndReason::ProtocolError => Self::InvalidInstruction,
            EndReason::Timeout => Self::Timeout,
            EndReason::HubShutdown => Self::HubShutdown,
        }
    }
}

/// How one tunnel ended: the cause, who ended it when someone did (`endSession`), and the
/// event message. The tunnel task records it once (D-9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct End {
    pub(super) cause: Cause,
    pub(super) by: Option<Actor>,
    pub(super) message: &'static str,
}

impl End {
    /// An end for `cause` with a more specific event message than the cause's own.
    pub(super) const fn explained(cause: Cause, message: &'static str) -> Self {
        Self {
            cause,
            by: None,
            message,
        }
    }
}

impl From<Cause> for End {
    fn from(cause: Cause) -> Self {
        Self::explained(cause, cause.message())
    }
}

impl From<EndCause> for End {
    /// The end `endSession` or shutdown asked for, with who asked.
    fn from(signal: EndCause) -> Self {
        let cause = Cause::from(signal.reason);
        Self {
            cause,
            by: signal.by,
            message: cause.message(),
        }
    }
}
