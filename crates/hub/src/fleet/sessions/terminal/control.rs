//! The terminal tunnel's text messages (K4 Section 8.4): the closed set the client may send,
//! parsed strictly, and the two the hub sends. Pure.
//!
//! A client message with another `type`, a missing, repeated or extra field, a value out of
//! range, a number that is not an integer, or more than [`MAX_TEXT_BYTES`] is refused. The
//! refusal names no part of the message (LOG-007).

use serde::Deserialize;

use crate::fleet::sessions::EndReason;

/// Largest text message from the client (K4 Section 8.5).
pub(super) const MAX_TEXT_BYTES: usize = 256;
/// Largest terminal size, in cells, either way (K4 Section 8.4).
const MAX_CELLS: u16 = 1000;

/// The whitespace JSON allows between tokens (RFC 8259 Section 2).
const JSON_SPACE: [char; 4] = [' ', '\t', '\n', '\r'];

/// What the hub sends when it has sent nothing for a while (K4 Section 8.4).
pub(super) const PING: &str = r#"{"type":"ping"}"#;

/// A terminal size in cells, each `1..=1000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Size {
    pub(super) cols: u16,
    pub(super) rows: u16,
}

/// A client text message the hub accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Control {
    /// The terminal's size: the first opens the PTY, later ones change it.
    Resize(Size),
    /// The client is alive; the hub does not answer.
    Ping,
}

/// Why a client text message was refused. Holds nothing from the message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BadControl {
    #[error("the text message is longer than 256 bytes")]
    TooLong,
    #[error("the text message is not one of the terminal's messages")]
    Unknown,
    #[error("a terminal size is outside 1 to 1000 cells")]
    OutOfRange,
}

/// The wire shape. `deny_unknown_fields` with struct variants refuses extra fields on both
/// messages (a unit variant would ignore them); serde refuses a repeated field, a missing
/// one, a float, a negative number and anything past `u16`. `parse` refuses the array form
/// serde would also read.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Wire {
    Resize { cols: u16, rows: u16 },
    Ping {},
}

/// The message in `text`, or why it is refused.
///
/// # Errors
/// [`BadControl::TooLong`] over [`MAX_TEXT_BYTES`], [`BadControl::Unknown`] for anything that
/// is not exactly one K4 Section 8.4 client message, [`BadControl::OutOfRange`] for a size
/// outside `1..=1000`.
pub(super) fn parse(text: &str) -> Result<Control, BadControl> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(BadControl::TooLong);
    }
    // serde also reads an internally tagged enum from an array (`["ping"]`); K4 Section 8.4
    // messages are objects only.
    if !text.trim_start_matches(JSON_SPACE).starts_with('{') {
        return Err(BadControl::Unknown);
    }
    match serde_json::from_str::<Wire>(text).map_err(|_| BadControl::Unknown)? {
        Wire::Resize { cols, rows } => {
            let cells = 1..=MAX_CELLS;
            if !cells.contains(&cols) || !cells.contains(&rows) {
                return Err(BadControl::OutOfRange);
            }
            Ok(Control::Resize(Size { cols, rows }))
        }
        Wire::Ping {} => Ok(Control::Ping),
    }
}

/// The `end` message for `reason`, with the K1 `SessionEndReason` name.
pub(super) fn end(reason: EndReason) -> String {
    format!(r#"{{"type":"end","reason":"{}"}}"#, k1_name(reason))
}

/// The K1 `SessionEndReason` value of `reason`, as GraphQL spells it.
pub(super) const fn k1_name(reason: EndReason) -> &'static str {
    match reason {
        EndReason::Operator => "OPERATOR",
        EndReason::RemoteClosed => "REMOTE_CLOSED",
        EndReason::GatewayError => "GATEWAY_ERROR",
        EndReason::AuthFailed => "AUTH_FAILED",
        EndReason::HostKeyMismatch => "HOST_KEY_MISMATCH",
        EndReason::ProtocolError => "PROTOCOL_ERROR",
        EndReason::Timeout => "TIMEOUT",
        EndReason::HubShutdown => "HUB_SHUTDOWN",
        EndReason::ServerCertificateMismatch => "SERVER_CERTIFICATE_MISMATCH",
    }
}
