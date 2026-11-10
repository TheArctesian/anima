//! Writing a recording: the header, then each part sealed in order, then the last part.

use super::{
    MAX_PART,
    header::{HEADER_LEN, Header},
    key::RecordingKey,
    part::{self, Place},
};

/// Why the sealer refused a part. The texts hold no recording content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SealError {
    /// A part before the last must hold at least one byte.
    #[error("a part before the last is empty")]
    EmptyPart,
    /// A part holds at most 64 KiB of plaintext.
    #[error("a part is larger than 64 KiB")]
    PartTooLarge,
    /// The part counter is spent: only the last part may take the final counter, and the
    /// counter never wraps.
    #[error("the recording has run out of part numbers")]
    TooManyParts,
}

/// Seals a recording's parts in order. [`Sealer::finish`] seals the last part and drops the
/// key, which wipes it; nothing can be sealed after it.
pub struct Sealer {
    key: RecordingKey,
    header: [u8; HEADER_LEN],
    counter: u32,
}

impl Sealer {
    pub fn new(key: RecordingKey, header: &Header) -> Self {
        Self {
            key,
            header: header.encode(),
            counter: 0,
        }
    }

    /// The header bytes, written at the start of the file before any part.
    pub const fn header(&self) -> &[u8; HEADER_LEN] {
        &self.header
    }

    /// Seals the next part before the last: 1 to 64 KiB of plaintext. Returns the framed
    /// bytes to append to the file.
    ///
    /// # Errors
    ///
    /// [`SealError`] when the part is empty or too large, or the counter has reached its
    /// last value, which only the last part may use.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
        if plaintext.is_empty() {
            return Err(SealError::EmptyPart);
        }
        check_size(plaintext)?;
        if self.counter == u32::MAX {
            return Err(SealError::TooManyParts);
        }
        let framed = self.seal_at(plaintext, false);
        self.counter += 1;
        Ok(framed)
    }

    /// Seals the last part: 0 to 64 KiB of plaintext, with the STREAM last flag.
    ///
    /// # Errors
    ///
    /// [`SealError::PartTooLarge`] when the part is too large.
    pub fn finish(self, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
        check_size(plaintext)?;
        Ok(self.seal_at(plaintext, true))
    }

    fn seal_at(&self, plaintext: &[u8], last: bool) -> Vec<u8> {
        let place = Place {
            counter: self.counter,
            last,
        };
        part::seal(&self.key, &self.header, place, plaintext)
    }

    /// Moves the counter, so a test reaches its end without sealing 2^32 parts.
    #[cfg(test)]
    pub(super) const fn at_counter(mut self, counter: u32) -> Self {
        self.counter = counter;
        self
    }
}

const fn check_size(plaintext: &[u8]) -> Result<(), SealError> {
    if plaintext.len() > MAX_PART {
        return Err(SealError::PartTooLarge);
    }
    Ok(())
}
