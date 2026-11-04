//! Which client instructions reach guacd (GW-011, GW-012, GW-013, GW-015, D-8).
//!
//! Only an allow-list of opcodes is forwarded, each with at least the arguments guacd's
//! handler reads: libguac indexes `argv` without checking `argc` (guacamole-server 1.5.5 and
//! 1.6.0 `src/libguac/user-handlers.c`; `guac_user_handle_instruction` in `user.c` passes
//! `argc` through unchecked), so a short instruction would make guacd read past the
//! arguments it got. Anything else closes the tunnel with 1002.

use guac_protocol::{Display, Instruction};

use super::decimal::decimal;
use super::ending::Cause;
use super::relay::{ClientStep, MAX_CLIENT_STREAMS, PROMPTABLE, Relay};

/// The client may always set the terminal colour scheme (GW-012, guacd SSH `argv`).
const COLOR_SCHEME: &str = "color-scheme";
/// Most digits in a `size` value: 8192 has four; one more allows a leading zero.
const SIZE_DIGITS: usize = 5;
/// Most digits in a stream index or an `ack` status.
const NUMBER_DIGITS: usize = 5;

/// Input opcodes forwarded as they are, with the number of arguments guacd reads.
const INPUT_ARITY: [(&str, usize); 5] = [
    ("key", 2),
    ("mouse", 3),
    ("touch", 7),
    ("sync", 1),
    ("nop", 0),
];

impl Relay {
    /// What to do with one decoded client instruction.
    pub(super) fn on_client(&mut self, i: Instruction) -> ClientStep {
        match i.opcode.as_str() {
            "" => internal(i),
            "size" => size(&i),
            "disconnect" => ClientStep::End {
                forward: Some(i),
                cause: Cause::OperatorClosed,
            },
            "ack" => ack(i),
            "clipboard" => self.open_stream(i, 2),
            "argv" => self.argv(i),
            "blob" => self.on_stream(i, 2, false),
            "end" => self.on_stream(i, 1, true),
            opcode => match INPUT_ARITY.iter().find(|(name, _)| *name == opcode) {
                Some(&(_, arity)) if i.args.len() >= arity => ClientStep::Forward(i),
                _ => invalid(),
            },
        }
    }

    /// `argv,<stream>,<mimetype>,<name>`: forwarded for `color-scheme`, or once for a
    /// credential guacd asked for with `required` (GW-015).
    fn argv(&mut self, i: Instruction) -> ClientStep {
        let Some(name) = i.args.get(2) else {
            return invalid();
        };
        let allowed = name == COLOR_SCHEME
            || PROMPTABLE
                .iter()
                .find(|prompt| **prompt == name.as_str())
                .is_some_and(|prompt| self.prompted.remove(prompt));
        if allowed {
            self.open_stream(i, 3)
        } else {
            invalid()
        }
    }

    /// An instruction that opens client stream `args[0]`; the stream may then carry `blob`
    /// and `end`.
    fn open_stream(&mut self, i: Instruction, arity: usize) -> ClientStep {
        match stream_index(&i, arity) {
            Some(stream) => {
                self.client_streams.insert(stream);
                ClientStep::Forward(i)
            }
            None => invalid(),
        }
    }

    /// `blob` or `end` on client stream `args[0]`, which must be open; `end` closes it.
    fn on_stream(&mut self, i: Instruction, arity: usize, closes: bool) -> ClientStep {
        let open = stream_index(&i, arity).filter(|stream| self.client_streams.contains(stream));
        match open {
            Some(stream) => {
                if closes {
                    self.client_streams.remove(&stream);
                }
                ClientStep::Forward(i)
            }
            None => invalid(),
        }
    }
}

/// The internal ping `0.,4.ping,<n>.<timestamp>;` is answered with itself and never reaches
/// guacd (GW-011); any other internal instruction is dropped.
fn internal(i: Instruction) -> ClientStep {
    if i.args.first().is_some_and(|first| first == "ping") {
        ClientStep::Reply(i)
    } else {
        ClientStep::Drop
    }
}

/// `size,<width>,<height>[,<dpi>]`, forwarded as plain numbers when width and height are
/// 1..=8192 and the DPI, when present, is 48..=480 (K4 Section 4, R-018); else dropped.
/// guacamole-common-js sends no DPI (WP-08 H-6).
fn size(i: &Instruction) -> ClientStep {
    let numbers: Option<Vec<u32>> = i.args.iter().map(|arg| decimal(arg, SIZE_DIGITS)).collect();
    let sides = Display::MIN_SIDE_PX..=Display::MAX_SIDE_PX;
    let dpi = Display::MIN_DPI..=Display::MAX_DPI;
    let fits = match numbers.as_deref() {
        Some([width, height]) => sides.contains(width) && sides.contains(height),
        Some([width, height, density]) => {
            sides.contains(width) && sides.contains(height) && dpi.contains(density)
        }
        _ => false,
    };
    match numbers {
        Some(numbers) if fits => {
            ClientStep::Forward(Instruction::new("size", numbers.iter().map(u32::to_string)))
        }
        _ => ClientStep::Drop,
    }
}

/// `ack,<stream>,<message>,<status>` for a stream guacd opened. Forwarded when the stream
/// and status are numbers; guacd ignores an `ack` for a stream it has not open.
fn ack(i: Instruction) -> ClientStep {
    let numbers = i.args.len() >= 3
        && decimal(&i.args[0], NUMBER_DIGITS).is_some()
        && decimal(&i.args[2], NUMBER_DIGITS).is_some();
    if numbers {
        ClientStep::Forward(i)
    } else {
        invalid()
    }
}

/// The client stream index in `args[0]`, when the instruction has `arity` arguments or more
/// and the index is below guacd's stream limit.
fn stream_index(i: &Instruction, arity: usize) -> Option<u16> {
    if i.args.len() < arity {
        return None;
    }
    let index = decimal(i.args.first()?, NUMBER_DIGITS)?;
    u16::try_from(index)
        .ok()
        .filter(|index| *index < MAX_CLIENT_STREAMS)
}

fn invalid() -> ClientStep {
    ClientStep::End {
        forward: None,
        cause: Cause::InvalidInstruction,
    }
}
