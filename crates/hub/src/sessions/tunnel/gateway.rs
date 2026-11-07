//! What guacd's instructions after `ready` mean for the session (K4 Sections 4 and 5).
//!
//! The policy keeps some of guacd's streams from the client: its `clipboard` when copy is
//! off, and a `file` or the `body` of a file when download is off. The opening instruction
//! and the stream's `blob` and `end` are dropped. guacd sends clipboard content without
//! waiting, but holds a download until the client's first `ack`, so a withheld download is
//! answered with a refusing `ack` and guacd frees the stream (guacamole-server 1.5.5 and
//! 1.6.0 `src/protocols/rdp/download.c`, `guac_rdp_download_ack_handler`).

use guac_protocol::Instruction;

use super::decimal::decimal;
use super::ending::Cause;
use super::relay::{GatewayStep, MAX_STREAMS, Relay};

/// `GUAC_PROTOCOL_STATUS_CLIENT_UNAUTHORIZED` (1.5.5 `src/libguac/guacamole/protocol-types.h:135`).
const CLIENT_UNAUTHORIZED: u32 = 0x0301;
/// `GUAC_PROTOCOL_STATUS_CLIENT_FORBIDDEN` (1.5.5 `src/libguac/guacamole/protocol-types.h:141`).
const CLIENT_FORBIDDEN: u32 = 0x0303;
/// `GUAC_PROTOCOL_STATUS_SERVER_ERROR`, sent when guacd's status is missing or not plain
/// decimal (1.5.5 `src/libguac/guacamole/protocol-types.h:59`).
const SERVER_ERROR: u32 = 0x0200;
/// Most digits in a status: the largest, 0x03FF, is 1023.
const STATUS_DIGITS: usize = 5;
/// Most digits in a stream index.
const STREAM_DIGITS: usize = 5;
/// The mimetype of a shared-drive folder listing (libguac `GUAC_USER_STREAM_INDEX_MIMETYPE`):
/// browsing needs only the drive, not download.
const LISTING: &str = "application/vnd.glyptodon.guacamole.stream-index+json";
/// What the hub tells guacd when it withholds a download.
const DOWNLOAD_REFUSED: &str = "Download is turned off for this device.";
/// The only `error` text the client gets: what guacd 1.5.5 and 1.6.0 send for every abort
/// (`src/libguac/client.c:413` and `:402`). Any other guacd text stays with the hub (D-11).
const ERROR_TEXT: &str = "Aborted. See logs.";

/// The internal instruction that tells guacamole-common-js the tunnel is open; the hub sends
/// it to the client before the first guacd instruction (K4 Section 4, WP-08 H-7).
/// `tunnel_id` is a new UUID, never guacd's connection ID (GW-014).
pub(super) fn tunnel_opened(tunnel_id: &str) -> Instruction {
    Instruction::new("", [tunnel_id])
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
                cause: Cause::RemoteClosed,
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
            "clipboard" if !self.policy.transfer.copy => self.withhold(&i, 0, false),
            "file" if !self.policy.downloads() => self.withhold(&i, 0, true),
            "body" if !self.may_send_body(&i) => self.withhold(&i, 1, true),
            "blob" => self.on_stream(i, false),
            "end" => self.on_stream(i, true),
            _ => GatewayStep::Forward(i),
        }
    }

    /// Remembers the credential names in a `required` that the session's protocol prompts
    /// for; other names are not answerable.
    fn note_prompts(&mut self, names: &[String]) {
        let prompts = self.policy.prompts();
        for name in names {
            if let Some(prompt) = prompts.iter().find(|prompt| **prompt == name.as_str()) {
                self.prompted.insert(prompt);
            }
        }
    }

    /// `body,<object>,<stream>,<mimetype>,<name>` answers the client's `get`: a folder
    /// listing needs the shared drive, a file's content needs download too.
    fn may_send_body(&self, i: &Instruction) -> bool {
        let listing = i.args.get(2).is_some_and(|mimetype| mimetype == LISTING);
        if listing {
            self.policy.shared_drive
        } else {
            self.policy.downloads()
        }
    }

    /// Keeps guacd stream `args[at]` from the client; `refuse` answers guacd with an `ack`.
    /// An index that is not a stream index is not tracked: the client never saw the stream
    /// open, and a stray `blob` is caught by [`Relay::on_stream`].
    fn withhold(&mut self, i: &Instruction, at: usize, refuse: bool) -> GatewayStep {
        let Some(stream) = i.args.get(at).and_then(|index| stream_index(index)) else {
            return GatewayStep::Drop;
        };
        self.withheld_streams.insert(stream);
        if refuse {
            let status = CLIENT_FORBIDDEN.to_string();
            let ack = [stream.to_string(), DOWNLOAD_REFUSED.to_owned(), status];
            GatewayStep::Refuse(Instruction::new("ack", ack))
        } else {
            GatewayStep::Drop
        }
    }

    /// guacd's `blob` or `end` on stream `args[0]`: dropped when the stream is withheld (and
    /// `end` forgets it), else forwarded. While any stream is withheld, one whose index is
    /// not a plain number is dropped too, as guacamole-common-js would read it as a number.
    fn on_stream(&mut self, i: Instruction, closes: bool) -> GatewayStep {
        let withheld = match i.args.first().and_then(|index| stream_index(index)) {
            Some(stream) if closes => self.withheld_streams.remove(&stream),
            Some(stream) => self.withheld_streams.contains(&stream),
            None => !self.withheld_streams.is_empty(),
        };
        if withheld {
            GatewayStep::Drop
        } else {
            GatewayStep::Forward(i)
        }
    }
}

/// A guacd stream index: a plain number below guacd's stream limit.
fn stream_index(text: &str) -> Option<u16> {
    let index = decimal(text, STREAM_DIGITS)?;
    u16::try_from(index).ok().filter(|index| *index < MAX_STREAMS)
}

/// The status of `error,<message>,<status>` when it is plain decimal, else server error.
fn error_status(i: &Instruction) -> u32 {
    i.args
        .get(1)
        .and_then(|status| decimal(status, STATUS_DIGITS))
        .unwrap_or(SERVER_ERROR)
}

/// 0x0301 and 0x0303 are sign-in failures; any other status is a gateway error.
fn error_cause(status: u32) -> Cause {
    match status {
        CLIENT_UNAUTHORIZED | CLIENT_FORBIDDEN => Cause::AuthFailed,
        _ => Cause::GatewayError,
    }
}
