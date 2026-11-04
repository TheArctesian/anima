//! What guacd's instructions after `ready` mean for the session (K4 Sections 4 and 5, D-9).
//!
//! # No host-key text on the wire
//!
//! guacd logs the host-key mismatch as "Host key did not match any provided known host
//! keys. %s" (guacamole-server 1.5.5 `src/common-ssh/ssh.c:552-553`, 1.6.0
//! `src/common-ssh/ssh.c:490-491`), but `guac_client_abort` sends every abort to the client
//! as `error` with the fixed text "Aborted. See logs." (1.5.5 `src/libguac/client.c:413`,
//! 1.6.0 `src/libguac/client.c:402`). The mismatch arrives as status 0x0200 (server error),
//! the same status as other guacd failures, so nothing here can tell it apart: the tunnel
//! task learns of a mismatch from the hub's own host-key check, not from guacd.

use guac_protocol::Instruction;

use super::ending::EndCause;
use super::relay::{GatewayStep, PROMPTABLE, Relay, decimal};

/// `GUAC_PROTOCOL_STATUS_CLIENT_UNAUTHORIZED` (1.5.5 `src/libguac/guacamole/protocol-types.h:135`).
const CLIENT_UNAUTHORIZED: u32 = 0x0301;
/// `GUAC_PROTOCOL_STATUS_CLIENT_FORBIDDEN` (1.5.5 `src/libguac/guacamole/protocol-types.h:141`).
const CLIENT_FORBIDDEN: u32 = 0x0303;
/// Most digits in a status: the largest, 0x03FF, is 1023.
const STATUS_DIGITS: usize = 5;

/// The `error` the hub sends to the client on a host-key mismatch, in place of anything from
/// guacd (K4 Section 5). The status is decimal text, as on the wire.
pub(super) fn host_key_changed() -> Instruction {
    Instruction::new(
        "error",
        ["Host key changed".to_owned(), CLIENT_FORBIDDEN.to_string()],
    )
}

impl Relay {
    /// What to do with one decoded guacd instruction after `ready`.
    pub(super) fn on_gateway(&mut self, i: Instruction) -> GatewayStep {
        match i.opcode.as_str() {
            "required" => {
                self.note_prompts(&i.args);
                GatewayStep::Forward(i)
            }
            "disconnect" => GatewayStep::End {
                send: i,
                cause: EndCause::RemoteClosed,
            },
            "error" => GatewayStep::End {
                cause: error_cause(&i),
                send: i,
            },
            // Internal instructions belong to the hub-client tunnel; one from guacd could
            // pass for the hub's own (K4 Section 4).
            "" => GatewayStep::Drop,
            _ => GatewayStep::Forward(i),
        }
    }

    /// Remembers the credential names in a `required`; other names are not answerable.
    fn note_prompts(&mut self, names: &[String]) {
        for name in names {
            if let Some(prompt) = PROMPTABLE.iter().find(|prompt| **prompt == name.as_str()) {
                self.prompted.insert(prompt);
            }
        }
    }
}

/// `error,<message>,<status>`: 0x0301 and 0x0303 are sign-in failures; any other status, or
/// one that is missing or not plain decimal, is a gateway error.
fn error_cause(i: &Instruction) -> EndCause {
    match i
        .args
        .get(1)
        .and_then(|status| decimal(status, STATUS_DIGITS))
    {
        Some(CLIENT_UNAUTHORIZED | CLIENT_FORBIDDEN) => EndCause::AuthFailed,
        _ => EndCause::GatewayError,
    }
}
