//! How a terminal session ended, and what `anima ssh` makes of it (K4 Sections 8.4 and 8.6).
//! Pure. The recorded `endReason` is authoritative; until it is known, the `end` message,
//! then the close code. A connection this side lost counts as lost whatever the hub recorded.

use crate::failure::Failure;
use crate::hub::{Code, HubError};

/// K1 `SessionEndReason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    Operator,
    RemoteClosed,
    GatewayError,
    AuthFailed,
    HostKeyMismatch,
    ProtocolError,
    Timeout,
    HubShutdown,
    ServerCertificateMismatch,
}

const NAMES: [(&str, EndReason); 9] = [
    ("OPERATOR", EndReason::Operator),
    ("REMOTE_CLOSED", EndReason::RemoteClosed),
    ("GATEWAY_ERROR", EndReason::GatewayError),
    ("AUTH_FAILED", EndReason::AuthFailed),
    ("HOST_KEY_MISMATCH", EndReason::HostKeyMismatch),
    ("PROTOCOL_ERROR", EndReason::ProtocolError),
    ("TIMEOUT", EndReason::Timeout),
    ("HUB_SHUTDOWN", EndReason::HubShutdown),
    (
        "SERVER_CERTIFICATE_MISMATCH",
        EndReason::ServerCertificateMismatch,
    ),
];

impl EndReason {
    /// The reason K1 names `name`, or `None` for a name this program does not know.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, reason)| *reason)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        NAMES
            .iter()
            .find(|(_, reason)| *reason == self)
            .map_or("", |(name, _)| name)
    }

    /// The single reason a close code stands for (K4 Section 8.6); 1000 and 1011 stand for
    /// more than one, so they give none.
    #[must_use]
    pub const fn of_close(code: u16) -> Option<Self> {
        match code {
            1001 => Some(Self::HubShutdown),
            1002 => Some(Self::ProtocolError),
            4001 => Some(Self::HostKeyMismatch),
            4002 => Some(Self::Timeout),
            4003 => Some(Self::ServerCertificateMismatch),
            _ => None,
        }
    }
}

/// Why this side's loop stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The hub closed the socket.
    Closed,
    /// Nothing came for 15 seconds, or a send stalled or failed.
    Lost,
    /// This side ended it: a signal (`SIGTERM`, `SIGHUP`, `SIGINT`) or a closed output.
    Left,
}

/// What the link saw at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ending {
    pub stop: Stop,
    /// The reason in the hub's `end` message, when one came.
    pub said: Option<EndReason>,
    /// The hub's close code, when it sent one.
    pub close: Option<u16>,
}

/// What the run ends with, given the `recorded` reason (from `endSession`) and the link's
/// `ending`, for the device named `device`.
///
/// # Errors
/// A [`Failure`] for every end the operator did not choose and the shell did not make.
pub fn verdict(recorded: Option<EndReason>, ending: Ending, device: &str) -> Result<(), Failure> {
    if ending.stop == Stop::Lost {
        return Err(unavailable(
            "The connection to the hub was lost; the session is over.",
        ));
    }
    let reason = recorded
        .or(ending.said)
        .or_else(|| ending.close.and_then(EndReason::of_close));
    match reason {
        None | Some(EndReason::Operator | EndReason::RemoteClosed) => Ok(()),
        Some(EndReason::HostKeyMismatch) => Err(Failure::Conflict {
            message: format!(
                "WARNING: the SSH host key of {device} is not the accepted one. Someone may be \
                 intercepting the connection, or the device was reinstalled. No credential was \
                 sent. If you expect the change, check the new key with `anima device test \
                 {device}` and accept it with `anima device trust accept-host-key {device}`."
            ),
        }),
        Some(EndReason::AuthFailed) => Err(refused(
            Code::Internal,
            format!(
                "The device refused the stored SSH credential, or none is stored. Set it with \
                 `anima device credential set ssh {device}`."
            ),
        )),
        Some(EndReason::GatewayError) => Err(unavailable(
            "The hub's SSH connection to the device failed.",
        )),
        Some(EndReason::Timeout) => Err(unavailable("The session timed out.")),
        Some(EndReason::HubShutdown) => Err(unavailable("The hub is shutting down.")),
        Some(EndReason::ProtocolError | EndReason::ServerCertificateMismatch) => Err(refused(
            Code::Internal,
            "The hub ended the session: it could not read this program's messages.".to_owned(),
        )),
    }
}

fn unavailable(message: &str) -> Failure {
    refused(Code::Unavailable, message.to_owned())
}

fn refused(code: Code, message: String) -> Failure {
    Failure::Hub(HubError::Refused {
        code,
        message,
        fields: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn closed(said: Option<EndReason>, close: Option<u16>) -> Ending {
        Ending {
            stop: Stop::Closed,
            said,
            close,
        }
    }

    fn exit(result: Result<(), Failure>) -> u8 {
        result.err().map_or(0, |failure| failure.exit_code())
    }

    #[test]
    fn k4_8_6_every_reason_name_reads_back() {
        for (name, reason) in NAMES {
            assert_eq!(EndReason::parse(name), Some(reason));
            assert_eq!(reason.as_str(), name);
        }
        assert_eq!(EndReason::parse("operator"), None);
    }

    #[test]
    fn k4_8_6_a_changed_host_key_warns_and_names_the_accept_command() {
        let failure = verdict(None, closed(None, Some(4001)), "web-1").expect_err("refused");
        let message = failure.to_string();
        assert!(message.contains("WARNING"), "{message}");
        assert!(
            message.contains("anima device trust accept-host-key web-1"),
            "{message}"
        );
        assert_ne!(failure.exit_code(), 0);
    }

    #[test]
    fn k4_8_4_the_recorded_reason_wins_then_end_then_the_close_code() {
        let ending = closed(Some(EndReason::GatewayError), Some(1011));
        assert_eq!(exit(verdict(Some(EndReason::RemoteClosed), ending, "d")), 0);
        assert_eq!(exit(verdict(None, ending, "d")), 7);
        assert_eq!(exit(verdict(None, closed(None, Some(4002)), "d")), 7);
        assert_eq!(exit(verdict(None, closed(None, Some(1000)), "d")), 0);
        let auth = closed(Some(EndReason::AuthFailed), Some(1011));
        assert_eq!(exit(verdict(None, auth, "d")), 1);
    }

    #[test]
    fn k4_8_4_a_lost_connection_fails_whatever_the_hub_recorded() {
        let lost = Ending {
            stop: Stop::Lost,
            said: None,
            close: None,
        };
        assert_eq!(exit(verdict(Some(EndReason::Operator), lost, "d")), 7);
    }
}
