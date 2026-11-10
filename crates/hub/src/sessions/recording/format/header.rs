//! The `.anrec` header (K4 §11): magic, format byte, recording ID and STREAM nonce prefix.
//!
//! Pure: the nonce prefix is a parameter (CQ-021); the caller draws it from the OS.

use uuid::Uuid;

use crate::ids::RecordingId;

/// The magic bytes every recording file starts with.
const MAGIC: &[u8; 7] = b"ANREC1\0";

/// Bytes in the STREAM nonce prefix: the 24-byte `XChaCha20` nonce less the 4-byte part
/// counter and the 1-byte last flag.
pub const NONCE_PREFIX_LEN: usize = 19;

/// Bytes in a header: magic, format byte, recording ID, nonce prefix.
pub const HEADER_LEN: usize = MAGIC.len() + 1 + 16 + NONCE_PREFIX_LEN;

/// What a recording holds, as its header's format byte names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Guacamole instructions as relayed to the owner (RDP and VNC). Byte `1`.
    Guacamole,
    /// asciicast v2 (SSH). Byte `2`.
    Asciicast,
}

impl Format {
    const fn byte(self) -> u8 {
        match self {
            Self::Guacamole => 1,
            Self::Asciicast => 2,
        }
    }

    const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Guacamole),
            2 => Some(Self::Asciicast),
            _ => None,
        }
    }
}

/// Why header bytes are not a recording header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    /// Not exactly [`HEADER_LEN`] bytes.
    #[error("the header has the wrong length")]
    Length,
    /// The bytes do not start with `ANREC1\0`.
    #[error("the header does not start with the recording magic")]
    Magic,
    /// The format byte names no format.
    #[error("the header names an unknown format")]
    UnknownFormat,
}

/// A recording file's header. Its encoded bytes are the associated data of every part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    format: Format,
    recording: RecordingId,
    nonce_prefix: [u8; NONCE_PREFIX_LEN],
}

impl Header {
    /// A header for a new recording; `nonce_prefix` is fresh random bytes.
    pub const fn new(
        format: Format,
        recording: RecordingId,
        nonce_prefix: [u8; NONCE_PREFIX_LEN],
    ) -> Self {
        Self {
            format,
            recording,
            nonce_prefix,
        }
    }

    pub const fn format(&self) -> Format {
        self.format
    }

    pub const fn recording(&self) -> RecordingId {
        self.recording
    }

    /// The header as it is written at the start of the file.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0_u8; HEADER_LEN];
        let (magic, rest) = bytes.split_at_mut(MAGIC.len());
        magic.copy_from_slice(MAGIC);
        let (format, rest) = rest.split_at_mut(1);
        format[0] = self.format.byte();
        let (id, prefix) = rest.split_at_mut(16);
        id.copy_from_slice(Uuid::from(self.recording).as_bytes());
        prefix.copy_from_slice(&self.nonce_prefix);
        bytes
    }

    /// Parses exactly [`HEADER_LEN`] bytes.
    ///
    /// # Errors
    ///
    /// [`HeaderError`] when the length, the magic or the format byte is wrong.
    pub fn parse(bytes: &[u8]) -> Result<Self, HeaderError> {
        let bytes: &[u8; HEADER_LEN] = bytes.try_into().map_err(|_| HeaderError::Length)?;
        let (magic, rest) = bytes.split_at(MAGIC.len());
        if magic != MAGIC {
            return Err(HeaderError::Magic);
        }
        let format = Format::from_byte(rest[0]).ok_or(HeaderError::UnknownFormat)?;
        let (id, prefix) = rest[1..].split_at(16);
        let id: [u8; 16] = id.try_into().expect("the split leaves 16 ID bytes");
        Ok(Self {
            format,
            recording: RecordingId::from(Uuid::from_bytes(id)),
            nonce_prefix: prefix
                .try_into()
                .expect("the split leaves the nonce prefix"),
        })
    }
}
