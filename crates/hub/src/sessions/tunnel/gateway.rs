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

use super::decimal::decimal;
use super::ending::EndCause;
use super::relay::{GatewayStep, PROMPTABLE, Relay};

/// `GUAC_PROTOCOL_STATUS_CLIENT_UNAUTHORIZED` (1.5.5 `src/libguac/guacamole/protocol-types.h:135`).
const CLIENT_UNAUTHORIZED: u32 = 0x0301;
/// `GUAC_PROTOCOL_STATUS_CLIENT_FORBIDDEN` (1.5.5 `src/libguac/guacamole/protocol-types.h:141`).
const CLIENT_FORBIDDEN: u32 = 0x0303;
/// `GUAC_PROTOCOL_STATUS_SERVER_ERROR`, sent when guacd's status is missing or not plain
/// decimal (1.5.5 `src/libguac/guacamole/protocol-types.h:59`).
const SERVER_ERROR: u32 = 0x0200;
/// Most digits in a status: the largest, 0x03FF, is 1023.
const STATUS_DIGITS: usize = 5;
/// The only `error` text the client gets: what guacd 1.5.5 and 1.6.0 send for every abort
/// (`src/libguac/client.c:413` and `:402`). Any other guacd text stays with the hub (D-11).
const ERROR_TEXT: &str = "Aborted. See logs.";

/// The internal instruction that tells guacamole-common-js the tunnel is open; the hub sends
/// it to the client before the first guacd instruction (K4 Section 4, WP-08 H-7).
/// `tunnel_id` is a new UUID, never guacd's connection ID (GW-014).
pub(super) fn tunnel_opened(tunnel_id: &str) -> Instruction {
    Instruction::new("", [tunnel_id])
}

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
            "error" => {
                let status = error_status(&i);
                GatewayStep::End {
                    send: Instruction::new("error", [ERROR_TEXT.to_owned(), status.to_string()]),
                    cause: error_cause(status),
                }
            }
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

/// The status of `error,<message>,<status>` when it is plain decimal, else server error.
fn error_status(i: &Instruction) -> u32 {
    i.args
        .get(1)
        .and_then(|status| decimal(status, STATUS_DIGITS))
        .unwrap_or(SERVER_ERROR)
}

/// 0x0301 and 0x0303 are sign-in failures; any other status is a gateway error.
fn error_cause(status: u32) -> EndCause {
    match status {
        CLIENT_UNAUTHORIZED | CLIENT_FORBIDDEN => EndCause::AuthFailed,
        _ => EndCause::GatewayError,
    }
}
