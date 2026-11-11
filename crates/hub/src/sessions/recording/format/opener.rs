//! Opening a recording one part at a time, without I/O: the caller reads the 4-byte length,
//! asks [`Opener::part_length`] how many bytes follow, reads them, and opens them. Playback
//! (K4 §12) can drive it from any reader, blocking or async; [`super::Reader`] drives it from
//! `std::io::Read`.

use zeroize::Zeroizing;

use super::{
    Damage, LENGTH_LEN, MAX_PART, TAG_LEN,
    header::{Format, HEADER_LEN, Header},
    key::RecordingKey,
    part,
};
use crate::ids::RecordingId;

/// A verified part.
pub struct Part {
    pub plaintext: Zeroizing<Vec<u8>>,
    /// The part carried the STREAM last flag: nothing may follow it.
    pub last: bool,
}

/// Opens the parts of one recording in order. A part opens only at its own counter, under
/// its own header and key, so a reordered, repeated, dropped or changed part fails as
/// [`Damage::Tag`].
pub struct Opener {
    key: RecordingKey,
    header: [u8; HEADER_LEN],
    format: Format,
    counter: u32,
    finished: bool,
}

impl Opener {
    /// Checks the header bytes, and that they name the `expected` recording.
    ///
    /// # Errors
    ///
    /// [`Damage::Header`] when the bytes are not a header, [`Damage::OtherRecording`] when
    /// they name another recording.
    pub fn new(key: RecordingKey, header: &[u8], expected: RecordingId) -> Result<Self, Damage> {
        let parsed = Header::parse(header)?;
        if parsed.recording() != expected {
            return Err(Damage::OtherRecording);
        }
        Ok(Self {
            key,
            header: parsed.encode(),
            format: parsed.format(),
            counter: 0,
            finished: false,
        })
    }

    pub const fn format(&self) -> Format {
        self.format
    }

    /// The last part has opened: the file must end here.
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// How many ciphertext bytes follow a part's length bytes, checked before the caller
    /// reads or allocates them.
    ///
    /// # Errors
    ///
    /// [`Damage::AfterLast`] when the last part has opened, [`Damage::Length`] when the
    /// length is shorter than a tag or longer than 64 KiB and a tag.
    pub fn part_length(&self, length: [u8; LENGTH_LEN]) -> Result<usize, Damage> {
        if self.finished {
            return Err(Damage::AfterLast);
        }
        let length = usize::try_from(u32::from_be_bytes(length)).map_err(|_| Damage::Length)?;
        check_length(length)?;
        Ok(length)
    }

    /// Opens the next part's ciphertext (with its tag).
    ///
    /// # Errors
    ///
    /// [`Damage`] when the last part has opened, the length is out of range, the tag fails,
    /// or a part before the last takes the counter's final value.
    pub fn open(&mut self, ciphertext: Vec<u8>) -> Result<Part, Damage> {
        if self.finished {
            return Err(Damage::AfterLast);
        }
        check_length(ciphertext.len())?;
        let (plaintext, last) = part::open(&self.key, &self.header, self.counter, ciphertext)?;
        if last {
            self.finished = true;
        } else if self.counter == u32::MAX {
            return Err(Damage::TooManyParts);
        } else {
            self.counter += 1;
        }
        Ok(Part { plaintext, last })
    }

    /// Moves the counter, so a test reaches its end without opening 2^32 parts.
    #[cfg(test)]
    pub(super) const fn at_counter(mut self, counter: u32) -> Self {
        self.counter = counter;
        self
    }
}

const fn check_length(length: usize) -> Result<(), Damage> {
    if length < TAG_LEN || length > MAX_PART + TAG_LEN {
        return Err(Damage::Length);
    }
    Ok(())
}
