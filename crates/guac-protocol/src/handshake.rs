//! The hub side of the guacd handshake as a pure state machine (GW-010).
//!
//! `select` → read `args` → send `size`, `audio`, `video`, `image`,
//! `timezone`, `name`, `connect` → read `ready`. Every refusal fails closed:
//! after the first error the handshake returns that error for every later
//! instruction, because the caller closes the gateway connection.

use crate::connect::{ConnectParams, Display};
use crate::decoder::{MAX_ELEMENTS, MAX_INSTRUCTION_BYTES};
use crate::instruction::{Instruction, encode, truncated};

/// The protocol version this crate implements and always answers with.
const VERSION: &str = "VERSION_1_5_0";
/// The oldest guacd version the hub accepts (K4 Section 3 step 2).
const MIN_VERSION: (u32, u32, u32) = (1, 5, 0);
/// Image formats the client can display.
const IMAGE_TYPES: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];

/// What the caller does after an instruction from guacd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeStep {
    /// Send these instructions to guacd, in order. Empty for a `nop`.
    ///
    /// `connect` carries passwords and keys: never log the instructions'
    /// arguments (LOG-007); `Instruction`'s `Debug` already omits them.
    Send(Vec<Instruction>),
    /// guacd is connected. Keep `connection_id` with the session and never
    /// send it to the client (GW-014).
    Ready { connection_id: String },
}

/// Why the handshake failed. No variant carries an argument value (LOG-007);
/// names and opcodes from guacd are cut to 32 characters.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    /// The first `args` value is missing, not `VERSION_X_Y_Z`, or older than 1.5.0.
    #[error("guacd does not offer protocol version 1.5.0 or later")]
    UnsupportedVersion,
    /// The display is outside the bounds of [`Display::within_bounds`].
    #[error("display size or DPI is out of bounds")]
    InvalidDisplay,
    /// A value to send holds a NUL byte, where guacd would cut it short.
    #[error("the {opcode} value for {argument:?} contains a NUL byte")]
    NulInValue {
        opcode: &'static str,
        argument: String,
    },
    /// An instruction to send is larger than guacd accepts.
    #[error(
        "the {opcode} instruction exceeds {MAX_INSTRUCTION_BYTES} bytes or {MAX_ELEMENTS} elements"
    )]
    InstructionTooLarge { opcode: String },
    /// guacd sent `ready` without a connection ID.
    #[error("guacd sent ready without a connection ID")]
    MissingConnectionId,
    /// guacd refused the connection with `error` before `ready`.
    #[error("guacd refused the connection (status {status:?}): {message:?}")]
    Gateway {
        /// The Guacamole status code, or `None` when guacd sent none or a non-number.
        status: Option<u16>,
        message: String,
    },
    /// guacd sent an instruction this step of the handshake does not accept.
    #[error("expected {expected} from guacd, got {opcode:?}")]
    UnexpectedInstruction {
        expected: &'static str,
        opcode: String,
    },
}

enum State {
    AwaitingArgs(ConnectParams),
    AwaitingReady,
    Ready,
    Failed(HandshakeError),
}

/// One handshake with guacd. No I/O: the caller sends and reads instructions.
///
/// No `Debug`: before `args` it holds the [`ConnectParams`] secrets.
pub struct Handshake {
    state: State,
}

impl Handshake {
    /// Starts a handshake for `protocol` (a hub constant such as `ssh`) and
    /// returns the `select` instruction to send first.
    #[must_use]
    pub fn new(protocol: &str, params: ConnectParams) -> (Self, Instruction) {
        let handshake = Self {
            state: State::AwaitingArgs(params),
        };
        (handshake, Instruction::new("select", [protocol]))
    }

    /// Advances the handshake with one instruction from guacd.
    ///
    /// A `nop` before `ready` returns `Send(vec![])`.
    ///
    /// # Errors
    ///
    /// Returns a [`HandshakeError`] when guacd refuses, offers an old version,
    /// sends an instruction out of order (or anything after `ready`), or when
    /// the reply to `args` would break guacd's limits. The handshake then
    /// returns the same error for every later instruction.
    pub fn on_instruction(&mut self, i: Instruction) -> Result<HandshakeStep, HandshakeError> {
        let state = std::mem::replace(&mut self.state, State::AwaitingReady);
        match advance(state, i) {
            Ok((next, step)) => {
                self.state = next;
                Ok(step)
            }
            Err(error) => {
                self.state = State::Failed(error.clone());
                Err(error)
            }
        }
    }
}

fn advance(state: State, i: Instruction) -> Result<(State, HandshakeStep), HandshakeError> {
    match (state, i.opcode.as_str()) {
        (State::Failed(error), _) => Err(error),
        (State::Ready, _) => Err(unexpected("nothing after ready", &i)),
        (state, "nop") => Ok((state, HandshakeStep::Send(Vec::new()))),
        (_, "error") => Err(gateway_error(&i)),
        (State::AwaitingArgs(params), "args") => {
            let send = reply_to_args(&params, &i.args)?;
            Ok((State::AwaitingReady, HandshakeStep::Send(send)))
        }
        (State::AwaitingArgs(_), _) => Err(unexpected("args", &i)),
        (State::AwaitingReady, "ready") => match i.args.into_iter().next() {
            Some(connection_id) if !connection_id.is_empty() => {
                Ok((State::Ready, HandshakeStep::Ready { connection_id }))
            }
            _ => Err(HandshakeError::MissingConnectionId),
        },
        (State::AwaitingReady, _) => Err(unexpected("ready", &i)),
    }
}

/// `size`, `audio`, `video`, `image`, `timezone`, `name` and `connect`, in order.
fn reply_to_args(
    params: &ConnectParams,
    args: &[String],
) -> Result<Vec<Instruction>, HandshakeError> {
    let (version, names) = args
        .split_first()
        .ok_or(HandshakeError::UnsupportedVersion)?;
    if parse_version(version).is_none_or(|v| v < MIN_VERSION) {
        return Err(HandshakeError::UnsupportedVersion);
    }
    if !params.display.within_bounds() {
        return Err(HandshakeError::InvalidDisplay);
    }
    let reply = vec![
        size(params.display),
        Instruction::new("audio", Vec::<String>::new()),
        Instruction::new("video", Vec::<String>::new()),
        Instruction::new("image", IMAGE_TYPES),
        Instruction::new(
            "timezone",
            [checked("timezone", "timezone", &params.timezone)?],
        ),
        Instruction::new("name", [checked("name", "name", &params.name)?]),
        connect(params, names)?,
    ];
    if let Some(large) = reply.iter().find(|i| !within_guacd_limits(i)) {
        return Err(HandshakeError::InstructionTooLarge {
            opcode: large.opcode.clone(),
        });
    }
    Ok(reply)
}

/// True when guacd's parser accepts `i` (its byte and element limits).
fn within_guacd_limits(i: &Instruction) -> bool {
    i.args.len() < MAX_ELEMENTS && encode(i).len() <= MAX_INSTRUCTION_BYTES
}

fn size(display: Display) -> Instruction {
    Instruction::new(
        "size",
        [display.width, display.height, display.dpi].map(|n| n.to_string()),
    )
}

/// `connect` with our version, then one value per `args` name, `""` when unset.
fn connect(params: &ConnectParams, names: &[String]) -> Result<Instruction, HandshakeError> {
    let mut values = Vec::with_capacity(names.len() + 1);
    values.push(VERSION.to_owned());
    for name in names {
        let value = params.values.get(name).unwrap_or_default();
        values.push(checked("connect", name, value)?);
    }
    Ok(Instruction::new("connect", values))
}

/// `value` as an owned argument, refused when guacd would cut it at a NUL.
fn checked(opcode: &'static str, argument: &str, value: &str) -> Result<String, HandshakeError> {
    if value.contains('\0') {
        return Err(HandshakeError::NulInValue {
            opcode,
            argument: truncated(argument),
        });
    }
    Ok(value.to_owned())
}

/// `(major, minor, patch)` from `VERSION_<major>_<minor>_<patch>`, digits only.
fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.strip_prefix("VERSION_")?.split('_').map(|part| {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse::<u32>().ok()
    });
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// guacd's `error` instruction is `error,<message>,<status>;`.
fn gateway_error(i: &Instruction) -> HandshakeError {
    HandshakeError::Gateway {
        status: i.args.get(1).and_then(|status| status.parse().ok()),
        message: i.args.first().cloned().unwrap_or_default(),
    }
}

fn unexpected(expected: &'static str, i: &Instruction) -> HandshakeError {
    HandshakeError::UnexpectedInstruction {
        expected,
        opcode: truncated(&i.opcode),
    }
}
