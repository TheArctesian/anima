//! How a live tunnel ends: the one table from cause to close code, end reason and event
//! severity, shared by the Guacamole and the terminal tunnel (K4 Sections 5 and 8.6, R-032,
//! WP-10 D-9).

use std::time::Duration;

use crate::events::Severity;
use crate::sessions::{Actor, EndCause, EndReason, Protocol};

/// The hub ends the session with `TIMEOUT` when the client has sent nothing for this long,
/// not even a ping; it is also the longest one send to the client may take (K4 Sections 4
/// and 8.5).
pub(in crate::sessions) const CLIENT_IDLE: Duration = Duration::from_secs(15);
/// The longest the hub waits for each peer while it closes: leaving the far side and sending
/// the client its close frame run side by side, so a stalled peer delays the end's write by
/// at most this, well inside `endSession`'s wait for the tunnel (D-10).
pub(in crate::sessions) const CLOSE_WAIT: Duration = Duration::from_secs(2);

/// One event sentence in the words of each protocol's session.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Sentences {
    ssh: &'static str,
    rdp: &'static str,
    vnc: &'static str,
}

impl Sentences {
    /// The sentence for a session over `protocol`.
    pub(in crate::sessions) const fn of(&self, protocol: Protocol) -> &'static str {
        match protocol {
            Protocol::Ssh => self.ssh,
            Protocol::Rdp => self.rdp,
            Protocol::Vnc => self.vnc,
        }
    }
}

/// `before`, the protocol's name, then `after`, for each protocol; or `same` for all.
macro_rules! naming_the_protocol {
    ($before:literal, $after:literal) => {
        &Sentences {
            ssh: concat!($before, "SSH", $after),
            rdp: concat!($before, "RDP", $after),
            vnc: concat!($before, "VNC", $after),
        }
    };
    (same $text:literal) => {
        &Sentences {
            ssh: $text,
            rdp: $text,
            vnc: $text,
        }
    };
}

/// An end's event message: one sentence for every protocol, or one naming the session's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum Words {
    Same(&'static str),
    Named(&'static Sentences),
}

impl Words {
    /// The message for a session over `protocol`.
    pub(in crate::sessions) const fn of(self, protocol: Protocol) -> &'static str {
        match self {
            Self::Same(text) => text,
            Self::Named(named) => named.of(protocol),
        }
    }
}

/// The event message when a send to the client stalls past [`CLIENT_IDLE`].
pub(in crate::sessions) const CLIENT_STOPPED_READING: &Sentences = naming_the_protocol!(
    "The client stopped reading what the hub sent, so the hub closed the ",
    " session."
);

/// The event message when the client left while the hub opened the far side.
pub(in crate::sessions) const CLIENT_LEFT: &Sentences =
    naming_the_protocol!("The client left before the ", " session opened.");

/// The event message when the tunnel task stopped without an end of its own.
pub(in crate::sessions) const TASK_STOPPED: &Sentences = naming_the_protocol!(
    "The hub's tunnel stopped unexpectedly and closed the ",
    " session."
);

/// Why a live tunnel ends. The first cause wins; the tunnel task records it once (D-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum Cause {
    /// The client closed the WebSocket or sent `disconnect`, or `endSession` ran.
    OperatorClosed,
    /// guacd sent `disconnect`, or its connection closed (EOF); on the terminal tunnel, the
    /// shell exited or the device closed the channel or the connection.
    RemoteClosed,
    /// guacd sent `error` with a status other than 0x0301 or 0x0303, or bytes that do not
    /// decode; on the terminal tunnel, the SSH connection failed or the PTY or shell was
    /// refused.
    GatewayError,
    /// guacd sent `error` with 0x0301 (unauthorized) or 0x0303 (forbidden); on the terminal
    /// tunnel, the device refused the credential or none is stored.
    AuthFailed,
    /// The device's host key is not the accepted one (SEC-007).
    HostKeyMismatch,
    /// The device's RDP server certificate is not the accepted one (SEC-007). Only the Guacamole
    /// tunnel ends with it.
    ServerCertificateMismatch,
    /// The client sent nothing for [`CLIENT_IDLE`], or stopped reading what the hub sends; on
    /// the terminal tunnel also no first `resize` in time.
    Timeout,
    /// The hub is shutting down.
    HubShutdown,
    /// The client broke the tunnel's protocol: on the Guacamole tunnel bytes that do not
    /// decode, an opcode outside the allow-list, an instruction whose shape guacd cannot
    /// handle or a binary message (GW-011); on the terminal tunnel a message outside K4
    /// Section 8.4; on both a message over the size cap or text that is not UTF-8 (R-039).
    InvalidInstruction,
}

/// What the tunnel task does at the end: the WebSocket close code for the client, the
/// session's `end_reason` and the severity of the `session.end` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Ending {
    pub(in crate::sessions) close_code: u16,
    pub(in crate::sessions) reason: EndReason,
    pub(in crate::sessions) severity: Severity,
}

impl Cause {
    /// The K4 Section 5 row for this cause; `InvalidInstruction` ends `PROTOCOL_ERROR` (R-034).
    pub(in crate::sessions) const fn ending(self) -> Ending {
        let (close_code, reason, severity) = match self {
            Self::OperatorClosed => (1000, EndReason::Operator, Severity::Info),
            Self::RemoteClosed => (1000, EndReason::RemoteClosed, Severity::Info),
            Self::GatewayError => (1011, EndReason::GatewayError, Severity::Warning),
            Self::AuthFailed => (1011, EndReason::AuthFailed, Severity::Warning),
            Self::HostKeyMismatch => (4001, EndReason::HostKeyMismatch, Severity::Error),
            Self::ServerCertificateMismatch => {
                (4003, EndReason::ServerCertificateMismatch, Severity::Error)
            }
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

    /// The event message for an end with this cause, naming the session's protocol; the
    /// terminal tunnel explains the causes whose words differ. Fixed text: it never holds
    /// guacd's words, an address, a value or session content (LOG-007, RA-008).
    pub(in crate::sessions) const fn message(self) -> &'static Sentences {
        match self {
            Self::OperatorClosed => naming_the_protocol!("The operator closed the ", " session."),
            Self::RemoteClosed => {
                naming_the_protocol!("The device or the gateway closed the ", " session.")
            }
            Self::GatewayError => {
                naming_the_protocol!("The gateway reported an error and closed the ", " session.")
            }
            Self::AuthFailed => naming_the_protocol!("The device refused the ", " sign-in."),
            Self::HostKeyMismatch => {
                naming_the_protocol!(same
                    "The device presented an SSH host key that is not the accepted one, so the \
                     hub closed the session before signing in. Test the connection and check the \
                     new key before accepting it."
                )
            }
            Self::ServerCertificateMismatch => {
                naming_the_protocol!(same
                    "The device presented an RDP server certificate that is not the accepted \
                     one, so the hub closed the session before signing in. Test the connection \
                     and check the new certificate before accepting it."
                )
            }
            Self::Timeout => naming_the_protocol!(
                "The client sent nothing for too long, so the hub closed the ",
                " session."
            ),
            Self::HubShutdown => {
                naming_the_protocol!("The hub shut down and closed the ", " session.")
            }
            Self::InvalidInstruction => naming_the_protocol!(
                "The client sent an instruction the hub does not accept, so the hub closed the ",
                " session."
            ),
        }
    }
}

/// A WebSocket error while reading the client: a message over the size cap or text that is
/// not UTF-8 breaks the protocol (R-039); anything else is the client leaving.
pub(in crate::sessions) fn unreadable(error: axum::Error) -> Cause {
    match error
        .into_inner()
        .downcast::<tungstenite::Error>()
        .as_deref()
    {
        Ok(tungstenite::Error::Capacity(_) | tungstenite::Error::Utf8(_)) => {
            Cause::InvalidInstruction
        }
        _ => Cause::OperatorClosed,
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
            EndReason::ServerCertificateMismatch => Self::ServerCertificateMismatch,
            EndReason::ProtocolError => Self::InvalidInstruction,
            EndReason::Timeout => Self::Timeout,
            EndReason::HubShutdown => Self::HubShutdown,
        }
    }
}

/// How one tunnel ended: the cause, who ended it when someone did (`endSession`), and the
/// event message. The tunnel task records it once (D-9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct End {
    pub(in crate::sessions) cause: Cause,
    pub(in crate::sessions) by: Option<Actor>,
    pub(in crate::sessions) message: Words,
}

impl End {
    /// An end for `cause` with a more specific event message than the cause's own, the same
    /// whatever the protocol.
    pub(in crate::sessions) const fn explained(cause: Cause, message: &'static str) -> Self {
        Self {
            cause,
            by: None,
            message: Words::Same(message),
        }
    }

    /// An end for `cause` with a more specific event message that names the protocol.
    pub(in crate::sessions) const fn worded(cause: Cause, message: &'static Sentences) -> Self {
        Self {
            cause,
            by: None,
            message: Words::Named(message),
        }
    }
}

impl From<Cause> for End {
    fn from(cause: Cause) -> Self {
        Self::worded(cause, cause.message())
    }
}

impl From<EndCause> for End {
    /// The end `endSession` or shutdown asked for, with who asked.
    fn from(signal: EndCause) -> Self {
        let cause = Cause::from(signal.reason);
        Self {
            cause,
            by: signal.by,
            message: Words::Named(cause.message()),
        }
    }
}
