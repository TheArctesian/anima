//! One transfer and its byte counts (K4 Section 9). Pure: it decides, the relay acts.
//!
//! An upload takes exactly its declared size from the client: a frame past it is a protocol
//! error. A download sends exactly the size the device gave when the file opened: the hub
//! reads at most what is left, and a device that sends more than asked, or ends the file
//! early, fails the transfer.

use super::protocol::DevicePath;

/// The most the hub asks the device for in one read, and so the largest download frame.
pub(super) const CHUNK: u32 = 32 * 1024;

/// Which way a transfer goes, as its audit entry names it (`file.upload`, `file.download`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Upload,
    Download,
}

/// How a transfer ended, as its audit entry records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Every byte moved and the device closed the file.
    Done,
    /// The hub refused the request (the reason is recorded beside it).
    Refused,
    /// The client cancelled, or the tunnel ended first.
    Cancelled,
    /// The device or the link failed mid-transfer.
    Failed,
}

impl Outcome {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Refused => "refused",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// A frame from the client would pass the upload's declared size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the client sent more bytes than the upload's size")]
pub(super) struct Overrun;

/// The device sent more than the hub asked for, or nothing before the file's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the device's file did not match the size it gave")]
pub(super) struct Mismatch;

/// One transfer: its path, the size it moves and how much has moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Transfer {
    pub(super) direction: Direction,
    pub(super) path: DevicePath,
    /// The upload's declared size, or the downloaded file's size once it is open (0 before).
    pub(super) size: u64,
    pub(super) moved: u64,
}

impl Transfer {
    pub(super) const fn new(direction: Direction, path: DevicePath, size: u64) -> Self {
        Self {
            direction,
            path,
            size,
            moved: 0,
        }
    }

    /// Bytes still to move.
    pub(super) const fn remaining(&self) -> u64 {
        self.size - self.moved
    }

    pub(super) const fn is_complete(&self) -> bool {
        self.moved == self.size
    }

    /// Counts a client frame of `len` bytes towards the declared size; returns where on the
    /// device it goes.
    ///
    /// # Errors
    /// [`Overrun`] when the frame would pass the size; nothing is counted then.
    pub(super) fn take(&mut self, len: usize) -> Result<u64, Overrun> {
        let len = u64::try_from(len).map_err(|_| Overrun)?;
        if len > self.remaining() {
            return Err(Overrun);
        }
        let offset = self.moved;
        self.moved += len;
        Ok(offset)
    }

    /// How much the next read asks for: [`CHUNK`] or what is left, if less.
    pub(super) fn next_read(&self) -> u32 {
        u32::try_from(self.remaining()).map_or(CHUNK, |left| left.min(CHUNK))
    }

    /// Counts `len` bytes the device gave for a read that asked for `asked`.
    ///
    /// # Errors
    /// [`Mismatch`] when it gave more than asked or than is left, or nothing while bytes
    /// were left.
    pub(super) fn add_read(&mut self, len: usize, asked: u32) -> Result<(), Mismatch> {
        let len = u64::try_from(len).map_err(|_| Mismatch)?;
        let past = len > u64::from(asked) || len > self.remaining();
        if past || (len == 0 && !self.is_complete()) {
            return Err(Mismatch);
        }
        self.moved += len;
        Ok(())
    }
}
