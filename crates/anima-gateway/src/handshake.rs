//! The gateway side of the Guacamole handshake (K4 Section 3, GW-010), as guacd answers
//! it: read `select`, send `args`, read `size`, `audio`, `video`, `image`, `timezone`,
//! `name` and `connect`, send `ready`. The hub's side is `guac_protocol::Handshake`.
//!
//! guacd sends `ready` before it reaches the device, and so does the gateway: a missing
//! credential is asked for after `ready` (`prompt`, GW-015), and a device that cannot be
//! reached or refuses the sign-in is an `error` in the open session.
//!
//! Pure: no I/O. No error and no `Debug` output holds a value from `connect` (LOG-007).

mod opening;
mod prompt;

use guac_protocol::Instruction;

use crate::frames::Formats;
pub use opening::{Opening, Secret};
pub use prompt::{Credential, Prompt, PromptStep};

/// The protocol the gateway serves.
pub const VNC: &str = "vnc";
/// The protocol version the gateway speaks; the hub answers with the same (K4 Section 3).
const VERSION: &str = "VERSION_1_5_0";
/// The `connect` values the gateway reads, in the order `args` lists them. They are the
/// names the hub sets for VNC (`tunnel/arguments.rs`); guacd's other VNC arguments have no
/// meaning here, and the hub sends an empty value for any name it is not offered.
const ARGUMENTS: [&str; 6] = [
    "hostname",
    "port",
    "username",
    "password",
    "disable-copy",
    "disable-paste",
];
/// The only text an `error` carries, as guacd's (K4 Section 4).
const ERROR_TEXT: &str = "Aborted. See logs.";

/// Why the gateway refuses a handshake. The hub gets `error` with [`Refused::status`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("the first instruction was not select")]
    NotSelect,
    #[error("the selected protocol is not served")]
    NotServed,
    #[error("an instruction came that the handshake does not have at this point")]
    Unexpected,
    #[error("connect came before a size")]
    NoSize,
    #[error("the value of {argument} cannot be used")]
    BadValue { argument: &'static str },
}

impl Refused {
    /// The Guacamole status: unsupported for a protocol the gateway does not serve, a bad
    /// request for the rest (`protocol-types.h`).
    #[must_use]
    pub const fn status(self) -> u16 {
        match self {
            Self::NotServed => 0x0100,
            _ => 0x0300,
        }
    }
}

/// `error` with the fixed text and `status`: what ends a handshake or a session.
#[must_use]
pub fn error(status: u16) -> Instruction {
    Instruction::new("error", [ERROR_TEXT.to_owned(), status.to_string()])
}

/// `ready` with the connection's ID. The caller makes the ID; the hub keeps it and never
/// shows it (GW-014).
#[must_use]
pub fn ready(connection_id: &str) -> Instruction {
    Instruction::new("ready", [connection_id])
}

/// What the caller does after an instruction from the hub.
#[derive(Debug)]
pub enum Step {
    /// Send this, then go on reading.
    Send(Instruction),
    /// Go on reading.
    Read,
    /// The hub sent `connect`: send [`ready`], then open this session.
    Opened(Box<Opening>),
}

/// What the hub has said so far.
enum State {
    AwaitingSelect,
    Collecting {
        size: Option<(usize, usize)>,
        formats: Formats,
    },
    Finished,
}

/// One handshake with the hub.
pub struct Handshake {
    state: State,
}

impl Default for Handshake {
    fn default() -> Self {
        Self {
            state: State::AwaitingSelect,
        }
    }
}

impl Handshake {
    /// Advances the handshake with one instruction from the hub.
    ///
    /// # Errors
    /// [`Refused`]: send [`error`] with its status and close. Every later instruction is
    /// refused too.
    pub fn on_instruction(&mut self, instruction: Instruction) -> Result<Step, Refused> {
        let state = std::mem::replace(&mut self.state, State::Finished);
        let (next, step) = advance(state, instruction)?;
        self.state = next;
        Ok(step)
    }
}

fn advance(state: State, instruction: Instruction) -> Result<(State, Step), Refused> {
    let collecting = |size, formats| State::Collecting { size, formats };
    match (state, instruction.opcode.as_str()) {
        (State::AwaitingSelect, "select") => {
            if instruction.args.first().map(String::as_str) != Some(VNC) {
                return Err(Refused::NotServed);
            }
            let args = Instruction::new("args", [VERSION].into_iter().chain(ARGUMENTS));
            Ok((collecting(None, Formats::default()), Step::Send(args)))
        }
        (State::AwaitingSelect, _) => Err(Refused::NotSelect),
        (State::Collecting { formats, .. }, "size") => {
            let size = opening::size(&instruction.args)?;
            Ok((collecting(Some(size), formats), Step::Read))
        }
        (State::Collecting { size, .. }, "image") => {
            let jpeg = instruction
                .args
                .iter()
                .any(|mimetype| mimetype == "image/jpeg");
            Ok((collecting(size, Formats { jpeg }), Step::Read))
        }
        (State::Collecting { size, formats }, "audio" | "video" | "timezone" | "name" | "nop") => {
            Ok((collecting(size, formats), Step::Read))
        }
        (State::Collecting { size, formats }, "connect") => {
            let size = size.ok_or(Refused::NoSize)?;
            let opening = Opening::from_connect(instruction.args, size, formats)?;
            Ok((State::Finished, Step::Opened(Box::new(opening))))
        }
        (State::Collecting { .. } | State::Finished, _) => Err(Refused::Unexpected),
    }
}
