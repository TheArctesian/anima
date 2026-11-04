//! The relay's view of one live tunnel after guacd's `ready` (K4 Section 4, D-7, D-8).
//!
//! Pure: the tunnel task decodes each instruction, asks [`Relay::on_client`] or
//! [`Relay::on_gateway`] (in `client.rs` and `gateway.rs`) what to do, and does only the
//! I/O. The relay remembers two things:
//!
//! - the credential names guacd asked for with `required` that the client has not answered
//!   (GW-015). The hub keeps the names only, never a value the operator types;
//! - the client's open input streams. A client `blob` or `end` is forwarded only on a stream
//!   the client opened with an allowed `clipboard` or `argv` and has not ended. Streams guacd
//!   opens (`img`, `clipboard`, `file`, …) are not tracked: the client only `ack`s them, and
//!   guacd ignores an `ack` for a stream it has not open (guacamole-server 1.5.5 and 1.6.0
//!   `src/libguac/user-handlers.c`, `__guac_handle_ack`), while tracking them would race
//!   with guacd's `end` and close sessions whose last `ack` arrives late.

use std::collections::BTreeSet;

use guac_protocol::Instruction;

use super::ending::EndCause;

/// What the tunnel task does with one client instruction.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ClientStep {
    /// Send this to guacd.
    Forward(Instruction),
    /// Send this back to the client, not to guacd (the internal ping, GW-011).
    Reply(Instruction),
    /// Send nothing (a `size` out of bounds, an internal instruction other than ping).
    Drop,
    /// Send `forward` to guacd if there is one, then end the session for `cause`.
    End {
        forward: Option<Instruction>,
        cause: EndCause,
    },
}

/// What the tunnel task does with one guacd instruction.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum GatewayStep {
    /// Send this to the client unchanged.
    Forward(Instruction),
    /// Send nothing.
    Drop,
    /// Send `send` to the client, then end the session for `cause`.
    End { send: Instruction, cause: EndCause },
}

/// The state of one tunnel's relay. Bounded: at most [`PROMPTABLE`] names and
/// [`MAX_CLIENT_STREAMS`] streams.
#[derive(Debug, Default)]
pub(super) struct Relay {
    pub(super) prompted: BTreeSet<&'static str>,
    pub(super) client_streams: BTreeSet<u16>,
}

/// The credential names a client may answer after guacd's `required` (GW-015). Any other
/// name, such as `hostname`, `port` or a path, stays hub-only even if guacd asks (GW-013).
/// guacd 1.5.5 and 1.6.0 SSH prompt in the terminal instead (`src/protocols/ssh/ssh.c`
/// `guac_terminal_prompt`); RDP and VNC send `required` for these names.
pub(super) const PROMPTABLE: [&str; 4] = ["username", "password", "passphrase", "domain"];

/// One past the largest client stream index guacd accepts: `GUAC_USER_MAX_STREAMS` is 512
/// in guacamole-server 1.6.0 and 64 in 1.5.5 (`src/libguac/guacamole/user-constants.h:38`);
/// 1.5.5 refuses 64..512 itself with an `ack`.
pub(super) const MAX_CLIENT_STREAMS: u16 = 512;

impl Relay {
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }
}

/// The internal instruction that tells guacamole-common-js the tunnel is open (K4 Section 4,
/// WP-08 H-7). `tunnel_id` is a new UUID, never guacd's connection ID (GW-014).
pub(super) fn tunnel_opened(tunnel_id: &str) -> Instruction {
    Instruction::new("", [tunnel_id])
}

/// A decimal number of 1..=`max_digits` ASCII digits; no sign, no `+`, no space.
/// `u32::from_str` alone would accept a leading `+`. `max_digits` is at most 9, so it fits.
pub(super) fn decimal(text: &str, max_digits: usize) -> Option<u32> {
    if text.is_empty() || text.len() > max_digits || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
