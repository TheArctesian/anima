//! The `.anrec` file format (K4 §11, D-13, G-2): pure, no file I/O.
//!
//! A file is the header ([`Header`]: `ANREC1\0`, a format byte, the 16-byte recording ID and
//! a 19-byte STREAM nonce prefix), then parts. Each part is a 4-byte big-endian ciphertext
//! length and an XChaCha20-Poly1305 STREAM (BE32) ciphertext of at most 64 KiB of plaintext;
//! the associated data of every part is the header bytes, and the last part carries the
//! STREAM last flag (`part`). [`Sealer`] writes the parts; [`Opener`] opens them without I/O
//! and [`Reader`] opens them from `std::io::Read`.
//!
//! Each recording has its own [`RecordingKey`], which no log or error can hold (LOG-007), and
//! no error text holds recording content or a path.

mod header;
mod key;
mod opener;
mod part;
mod reader;
mod sealer;
#[cfg(test)]
mod tests;

pub use header::{Format, HEADER_LEN, Header, HeaderError, NONCE_PREFIX_LEN};
pub use key::RecordingKey;
pub use opener::Opener;
pub use reader::{ReadError, Reader, Step};
pub use sealer::{SealError, Sealer};

/// The most plaintext one part holds (K4 §11).
pub const MAX_PART: usize = 64 * 1024;

/// Bytes in a part's length.
pub const LENGTH_LEN: usize = 4;

/// Bytes in a Poly1305 tag, at the end of each part's ciphertext.
const TAG_LEN: usize = 16;

/// Why the bytes of a recording cannot be trusted past some point. Every change to a file
/// falls out as one of these: the counter and the associated data bind each part to its
/// place, its file and its flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Damage {
    #[error("the header is not valid: {0}")]
    Header(#[from] HeaderError),
    /// The header names another recording than the one asked for.
    #[error("the file belongs to another recording")]
    OtherRecording,
    /// A part's length is shorter than a tag or longer than 64 KiB and a tag.
    #[error("a part has a length out of range")]
    Length,
    /// The file ends inside a part's length or ciphertext.
    #[error("the file ends inside a part")]
    Truncated,
    /// A part fails authentication: changed, moved, repeated, from another file or key.
    #[error("a part failed authentication")]
    Tag,
    /// Bytes follow the last part.
    #[error("bytes follow the last part")]
    AfterLast,
    /// A part before the last takes the counter's final value, so no last part could follow.
    #[error("a part lies past the part counter's range")]
    TooManyParts,
}
