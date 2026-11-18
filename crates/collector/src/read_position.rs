//! Where the read of a source file continues, so that only new text is sent (COL-004).

/// The count of bytes of a file that the hub has stored. It moves only after the hub accepts
/// the bytes, so text that was not sent stays in the file and is read again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadPosition(u64);

impl ReadPosition {
    /// A position that a store kept.
    #[must_use]
    pub fn at(offset: u64) -> Self {
        Self(offset)
    }

    /// The byte offset at which the next read starts.
    #[must_use]
    pub fn offset(self) -> u64 {
        self.0
    }

    /// The position in a file that now has `len` bytes. A file that is shorter than the
    /// position was made again, so its read starts at the start.
    #[must_use]
    pub fn in_file(self, len: u64) -> Self {
        if len < self.0 { Self(0) } else { self }
    }

    /// The position after the hub stored `sent`, the bytes that were read from this position.
    #[must_use]
    pub fn after(self, sent: &[u8]) -> Self {
        Self(self.0.saturating_add(sent.len() as u64))
    }
}

/// The lines of `read` that are complete: all bytes up to the last line feed, with it. The
/// writer of the file has not finished the text after it, so that text waits for the next read.
#[must_use]
pub fn whole_lines(read: &[u8]) -> &[u8] {
    let end = read
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |last| last + 1);
    &read[..end]
}
