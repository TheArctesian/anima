//! Reading a recording from `std::io::Read`, one verified part at a time, then how it ended.

use std::io::{self, Read};

use zeroize::Zeroizing;

use super::{
    Damage, LENGTH_LEN, header::Format, header::HEADER_LEN, key::RecordingKey, opener::Opener,
};
use crate::ids::RecordingId;

/// What the next read found.
pub enum Step {
    /// The plaintext of the next verified part; never empty.
    Part(Zeroizing<Vec<u8>>),
    /// The last part verified and nothing follows it.
    Done,
    /// The file ends cleanly between parts with no last part: a hub that stopped before it
    /// wrote one (K4 §11), or a file cut at a part boundary, which reads the same. The parts
    /// read so far verified; only `Done` proves the whole recording.
    Incomplete,
}

/// Why reading stopped short. The texts hold no recording content, key or path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    #[error("the recording is damaged: {0}")]
    Damaged(#[from] Damage),
    #[error("the recording could not be read: {0}")]
    Read(io::ErrorKind),
    /// The reader already returned its end or an error.
    #[error("the recording has already been read to its end")]
    Ended,
}

/// Reads one recording's parts from `source`, which yields the file from its first byte.
pub struct Reader<R> {
    source: R,
    opener: Opener,
    ended: bool,
}

impl<R: Read> Reader<R> {
    /// Reads and checks the header.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] when the header is short or wrong or names another recording,
    /// [`ReadError::Read`] when the source fails.
    pub fn open(
        mut source: R,
        key: RecordingKey,
        expected: RecordingId,
    ) -> Result<Self, ReadError> {
        let mut header = [0_u8; HEADER_LEN];
        let read = fill(&mut source, &mut header)?;
        let opener = Opener::new(key, &header[..read], expected)?;
        Ok(Self {
            source,
            opener,
            ended: false,
        })
    }

    pub const fn format(&self) -> Format {
        self.opener.format()
    }

    /// The next verified part's plaintext, or how the recording ended. After an end or an
    /// error every call returns [`ReadError::Ended`].
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] when a part fails (length, tag, truncation, bytes after the
    /// last part), [`ReadError::Read`] when the source fails.
    pub fn next_part(&mut self) -> Result<Step, ReadError> {
        if self.ended {
            return Err(ReadError::Ended);
        }
        let step = self.step();
        self.ended = !matches!(step, Ok(Step::Part(_)));
        step
    }

    fn step(&mut self) -> Result<Step, ReadError> {
        while !self.opener.is_finished() {
            let mut length = [0_u8; LENGTH_LEN];
            match fill(&mut self.source, &mut length)? {
                0 => return Ok(Step::Incomplete),
                LENGTH_LEN => {}
                _ => return Err(Damage::Truncated.into()),
            }
            let mut ciphertext = vec![0_u8; self.opener.part_length(length)?];
            if fill(&mut self.source, &mut ciphertext)? != ciphertext.len() {
                return Err(Damage::Truncated.into());
            }
            let part = self.opener.open(ciphertext)?;
            if !part.plaintext.is_empty() {
                return Ok(Step::Part(part.plaintext));
            }
        }
        match fill(&mut self.source, &mut [0_u8; 1])? {
            0 => Ok(Step::Done),
            _ => Err(Damage::AfterLast.into()),
        }
    }
}

/// Reads until `buffer` is full or the source ends; returns how many bytes it read.
fn fill(source: &mut impl Read, buffer: &mut [u8]) -> Result<usize, ReadError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match source.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(ReadError::Read(error.kind())),
        }
    }
    Ok(filled)
}
