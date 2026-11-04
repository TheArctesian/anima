//! The Guacamole protocol codec (`encode`, `Decoder`) and the hub side of the
//! guacd handshake (`Handshake`), with no I/O and no async.

mod connect;
mod decoder;
mod handshake;
mod instruction;

pub use connect::{ArgValues, ConnectParams, Display, is_zone_name};
pub use decoder::{DecodeError, Decoder, MAX_ELEMENTS, MAX_INSTRUCTION_BYTES, MAX_LENGTH_DIGITS};
pub use handshake::{Handshake, HandshakeError, HandshakeStep};
pub use instruction::{Instruction, encode};
