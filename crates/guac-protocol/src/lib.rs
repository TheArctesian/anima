//! Guacamole protocol parse and encode, no I/O and no async.

mod decoder;
mod instruction;

pub use decoder::{DecodeError, Decoder, MAX_ELEMENTS, MAX_INSTRUCTION_BYTES, MAX_LENGTH_DIGITS};
pub use instruction::{Instruction, encode};
