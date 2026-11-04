//! Incremental decoder for the Guacamole wire format (GW-011).
//!
//! The decoder reads one byte at a time, so a feed may end anywhere: inside a
//! length prefix, inside a value, or inside a multibyte UTF-8 character. It
//! keeps at most [`MAX_INSTRUCTION_BYTES`] of an incomplete instruction.

use std::fmt;

use crate::instruction::Instruction;

/// Largest encoded instruction, in bytes, including lengths and separators.
///
/// This is guacd's own parser limit (`GUAC_INSTRUCTION_MAX_LENGTH`), so guacd
/// never accepts a larger instruction and the hub never needs to buffer one.
pub const MAX_INSTRUCTION_BYTES: usize = 8192;

/// Largest number of elements in one instruction, the opcode included.
///
/// This is guacd's own parser limit (`GUAC_INSTRUCTION_MAX_ELEMENTS`).
pub const MAX_ELEMENTS: usize = 128;

/// Largest number of decimal digits in one element length.
///
/// This is guacd's own parser limit (`GUAC_INSTRUCTION_MAX_DIGITS`).
pub const MAX_LENGTH_DIGITS: usize = 5;

/// Why a byte stream is not a valid Guacamole instruction stream.
///
/// Positions are zero-based byte offsets in the whole stream fed to the
/// decoder. No variant carries instruction content (LOG-007).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("expected a length digit or '.' at byte {position}")]
    InvalidLength { position: u64 },
    #[error("element length at byte {position} has more than {limit} digits")]
    TooManyLengthDigits { position: u64, limit: usize },
    #[error("expected ',' or ';' after an element at byte {position}")]
    MissingSeparator { position: u64 },
    #[error("element ending at byte {position} is not valid UTF-8")]
    InvalidUtf8 { position: u64 },
    #[error("instruction exceeds {limit} bytes at byte {position}")]
    InstructionTooLong { position: u64, limit: usize },
    #[error("instruction exceeds {limit} elements at byte {position}")]
    TooManyElements { position: u64, limit: usize },
}

/// Where the decoder is inside the current element.
#[derive(Debug, Clone, Copy)]
enum Element {
    /// Reading the decimal length before `.`.
    Length { length: usize, digits: usize },
    /// Reading the value; `chars` code points have started so far.
    Value { length: usize, chars: usize },
}

/// Turns a byte stream into complete instructions, across arbitrary splits.
///
/// After the first error the decoder stays failed: every later `feed` returns
/// the same error, because the tunnel closes (GW-011).
///
/// `Debug` prints the stream position and the error only, never buffered bytes.
pub struct Decoder {
    element: Element,
    value: Vec<u8>,
    elements: Vec<String>,
    instruction_bytes: usize,
    position: u64,
    failed: Option<DecodeError>,
}

impl fmt::Debug for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decoder")
            .field("position", &self.position)
            .field("instruction_bytes", &self.instruction_bytes)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            element: Element::Length {
                length: 0,
                digits: 0,
            },
            value: Vec::new(),
            elements: Vec::new(),
            instruction_bytes: 0,
            position: 0,
            failed: None,
        }
    }

    /// Consumes `bytes` and returns every instruction they complete, in order.
    ///
    /// Bytes of an incomplete instruction are kept for the next call.
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] when the stream breaks the grammar or a limit.
    /// Instructions completed earlier in the same call are dropped, and the
    /// decoder returns the same error from then on.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Instruction>, DecodeError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        let mut complete = Vec::new();
        for &byte in bytes {
            match self.step(byte) {
                Ok(Some(instruction)) => complete.push(instruction),
                Ok(None) => {}
                Err(error) => {
                    self.release();
                    self.failed = Some(error.clone());
                    return Err(error);
                }
            }
            self.position = self.position.saturating_add(1);
        }
        Ok(complete)
    }

    fn step(&mut self, byte: u8) -> Result<Option<Instruction>, DecodeError> {
        self.instruction_bytes = self.instruction_bytes.saturating_add(1);
        if self.instruction_bytes > MAX_INSTRUCTION_BYTES {
            return Err(DecodeError::InstructionTooLong {
                position: self.position,
                limit: MAX_INSTRUCTION_BYTES,
            });
        }
        match self.element {
            Element::Length { length, digits } => {
                self.length_byte(byte, length, digits).map(|()| None)
            }
            Element::Value { length, chars } if chars == length && starts_char(byte) => {
                self.end_element(byte)
            }
            Element::Value { length, chars } => {
                let chars = if starts_char(byte) {
                    chars.saturating_add(1)
                } else {
                    chars
                };
                self.element = Element::Value { length, chars };
                self.value.push(byte);
                Ok(None)
            }
        }
    }

    fn length_byte(&mut self, byte: u8, length: usize, digits: usize) -> Result<(), DecodeError> {
        match byte {
            b'0'..=b'9' if digits >= MAX_LENGTH_DIGITS => Err(DecodeError::TooManyLengthDigits {
                position: self.position,
                limit: MAX_LENGTH_DIGITS,
            }),
            b'0'..=b'9' => {
                let length = length
                    .saturating_mul(10)
                    .saturating_add(usize::from(byte - b'0'));
                self.element = Element::Length {
                    length,
                    digits: digits.saturating_add(1),
                };
                Ok(())
            }
            b'.' if digits > 0 => {
                self.element = Element::Value { length, chars: 0 };
                Ok(())
            }
            _ => Err(DecodeError::InvalidLength {
                position: self.position,
            }),
        }
    }

    fn end_element(&mut self, separator: u8) -> Result<Option<Instruction>, DecodeError> {
        let position = self.position;
        if separator != b',' && separator != b';' {
            return Err(DecodeError::MissingSeparator { position });
        }
        let value = String::from_utf8(std::mem::take(&mut self.value))
            .map_err(|_| DecodeError::InvalidUtf8 { position })?;
        self.elements.push(value);
        self.element = Element::Length {
            length: 0,
            digits: 0,
        };
        if separator == b',' {
            if self.elements.len() >= MAX_ELEMENTS {
                return Err(DecodeError::TooManyElements {
                    position,
                    limit: MAX_ELEMENTS,
                });
            }
            return Ok(None);
        }
        self.instruction_bytes = 0;
        let mut elements = std::mem::take(&mut self.elements).into_iter();
        let opcode = elements.next().unwrap_or_default();
        Ok(Some(Instruction {
            opcode,
            args: elements.collect(),
        }))
    }

    /// Drops the partial instruction so a failed decoder holds no content.
    fn release(&mut self) {
        self.value = Vec::new();
        self.elements = Vec::new();
    }
}

/// True for a byte that starts a code point (anything but a UTF-8 continuation byte).
fn starts_char(byte: u8) -> bool {
    byte & 0xC0 != 0x80
}
