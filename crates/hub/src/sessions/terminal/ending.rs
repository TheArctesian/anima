//! How a terminal session ends, in the terminal's words (K4 Section 8.6): the shared end
//! table (`end::cause`) with event messages that name the device and the shell, not a
//! gateway. Pure. Every message is a fixed sentence with no device text, address, value or
//! session content (LOG-007, RA-008).

use crate::sessions::end::{Cause, End};

/// The request did not offer the `anima-terminal` subprotocol.
pub(super) const NOT_TERMINAL: &str = "The client did not open the tunnel as a terminal \
                                       WebSocket, so the hub closed the SSH session.";
/// No credential is stored for the device (D-3).
pub(super) const NO_CREDENTIAL: &str = "No SSH credential is stored for the device, so the hub \
                                        could not sign in. Store one in the device's SSH \
                                        settings.";
/// The stored private key or its passphrase does not decode.
pub(super) const UNREADABLE_KEY: &str = "The stored SSH private key or its passphrase cannot be \
                                         read, so the hub could not sign in.";
/// The client's first message was not its terminal size in time.
pub(super) const NO_SIZE: &str = "The client did not send its terminal size in time, so the \
                                  hub closed the SSH session.";
/// The device refused the PTY request.
pub(super) const PTY_REFUSED: &str =
    "The device refused a terminal for the session, so the hub closed the SSH session.";
/// The device refused the shell request.
pub(super) const SHELL_REFUSED: &str =
    "The device refused to start a shell, so the hub closed the SSH session.";
/// The device did not take a PTY or shell request, or did not answer it, in time.
pub(super) const NO_ANSWER: &str = "The device did not answer the hub's request for a terminal \
                                    or shell in time, so the hub closed the SSH session.";
/// Client input waited on the device past the request wait.
pub(super) const DEVICE_STOPPED_READING: &str = "The device stopped reading what the operator \
                                                 typed, so the hub closed the SSH session.";

/// The shell exited, or the device closed the channel or the connection.
const REMOTE_CLOSED: &str = "The shell exited or the device closed the SSH session.";
/// The SSH connection failed after the opening.
const CONNECTION_LOST: &str =
    "The SSH connection to the device failed, so the hub closed the session.";
/// A client message outside K4 Section 8.4 or over a limit of Section 8.5.
const BAD_MESSAGE: &str =
    "The client sent a message the hub does not accept, so the hub closed the SSH session.";

/// The end for `cause` with the terminal's event message.
pub(super) const fn ended(cause: Cause) -> End {
    match cause {
        Cause::RemoteClosed => End::explained(cause, REMOTE_CLOSED),
        Cause::GatewayError => End::explained(cause, CONNECTION_LOST),
        Cause::InvalidInstruction => End::explained(cause, BAD_MESSAGE),
        Cause::OperatorClosed
        | Cause::AuthFailed
        | Cause::HostKeyMismatch
        | Cause::ServerCertificateMismatch
        | Cause::Timeout
        | Cause::HubShutdown => End::worded(cause, cause.message()),
    }
}
