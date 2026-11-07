//! The relay's view of one live tunnel after guacd's `ready` (K4 Section 4, D-7, D-8).
//!
//! Pure: the tunnel task decodes each instruction, asks [`Relay::on_client`] or
//! [`Relay::on_gateway`] (in `client.rs` and `gateway.rs`) what to do, and does only the
//! I/O. The relay keeps the session's [`Policy`] and remembers:
//!
//! - the credential names guacd asked for with `required` that the client has not answered
//!   (GW-015). The hub keeps the names only, never a value the operator types;
//! - the client's open input streams. A client `blob` or `end` is forwarded only on a stream
//!   the client opened with an allowed `clipboard`, `argv`, `audio`, `file` or `put` and has
//!   not ended;
//! - the client streams opened by a `clipboard` the policy drops, whose `blob` and `end` are
//!   dropped too;
//! - the guacd streams the policy keeps from the client (`clipboard` with copy off, `file`
//!   and `body` with download off), whose `blob` and `end` are dropped. Other streams guacd
//!   opens (`img`, `audio`, …) are not tracked: the client only `ack`s them, and guacd ignores
//!   an `ack` for a stream it has not open (guacamole-server 1.5.5 and 1.6.0
//!   `src/libguac/user-handlers.c`, `__guac_handle_ack`), while tracking them would race
//!   with guacd's `end` and close sessions whose last `ack` arrives late.

use std::collections::BTreeSet;

use guac_protocol::Instruction;

use super::ending::Cause;
use super::policy::Policy;

/// What the tunnel task does with one client instruction.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ClientStep {
    /// Send this to guacd.
    Forward(Instruction),
    /// Send this back to the client, not to guacd (the internal ping, GW-011).
    Reply(Instruction),
    /// Send nothing (a `size` out of bounds, an internal instruction other than ping, a
    /// clipboard the policy refuses).
    Drop,
    /// Send `forward` to guacd if there is one, then end the session for `cause`.
    End {
        forward: Option<Instruction>,
        cause: Cause,
    },
}

/// What the tunnel task does with one guacd instruction.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum GatewayStep {
    /// Send this to the client unchanged.
    Forward(Instruction),
    /// Send nothing.
    Drop,
    /// Send nothing to the client and this `ack` back to guacd, which then closes a stream
    /// that waits for the client's `ack` before it sends any content.
    Refuse(Instruction),
    /// Send `send` to the client, then end the session for `cause`.
    End { send: Instruction, cause: Cause },
}

/// The state of one tunnel's relay. Bounded: at most the policy's prompt names and
/// [`MAX_STREAMS`] indices in each stream set.
#[derive(Debug)]
pub(super) struct Relay {
    pub(super) policy: Policy,
    pub(super) prompted: BTreeSet<&'static str>,
    pub(super) client_streams: BTreeSet<u16>,
    /// Client streams whose `clipboard` the policy dropped.
    pub(super) dropped_client_streams: BTreeSet<u16>,
    /// guacd streams the policy keeps from the client.
    pub(super) withheld_streams: BTreeSet<u16>,
}

impl Relay {
    /// A relay with nothing prompted and no stream open, under `policy`.
    pub(super) const fn new(policy: Policy) -> Self {
        Self {
            policy,
            prompted: BTreeSet::new(),
            client_streams: BTreeSet::new(),
            dropped_client_streams: BTreeSet::new(),
            withheld_streams: BTreeSet::new(),
        }
    }
}

/// One past the largest stream index guacd uses for a user: `GUAC_USER_MAX_STREAMS` is 512
/// in guacamole-server 1.6.0 and 64 in 1.5.5 (`src/libguac/guacamole/user-constants.h:38`);
/// 1.5.5 refuses client streams 64..512 itself with an `ack`.
pub(super) const MAX_STREAMS: u16 = 512;
