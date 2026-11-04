//! Guacamole protocol parse and encode, no I/O and no async.

mod connect;
mod decoder;
mod handshake;
mod instruction;

pub use connect::{ArgValues, ConnectParams, Display};
pub use decoder::{DecodeError, Decoder, MAX_ELEMENTS, MAX_INSTRUCTION_BYTES, MAX_LENGTH_DIGITS};
pub use handshake::{Handshake, HandshakeError, HandshakeStep};
pub use instruction::{Instruction, encode};
