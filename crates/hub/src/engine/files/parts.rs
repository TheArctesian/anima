//! Reading and writing the parts of a piece as streams (FIL-005): at most two parts are in
//! memory at a time, never a whole file.

use std::{io, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _, stream};
use zeroize::Zeroizing;

use super::stream::{Binding, PART, RECORD, StreamError};
use crate::engine::data_keys::DataKey;

/// The bytes of a sealed piece, as the store gives and takes them.
pub type SealedStream = Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

/// The plain bytes of a piece, part by part.
pub type PlainStream = Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

/// Gives the stored parts of a piece one at a time, each with its last-part mark.
pub(super) struct Records {
    sealed: SealedStream,
    buffer: Vec<u8>,
    ended: bool,
}

impl Records {
    pub(super) fn new(sealed: SealedStream) -> Self {
        Self {
            sealed,
            buffer: Vec::with_capacity(RECORD + 1),
            ended: false,
        }
    }

    /// Reads until `len` bytes are buffered or the stream ends.
    async fn fill(&mut self, len: usize) -> io::Result<()> {
        while self.buffer.len() < len && !self.ended {
            match self.sealed.next().await {
                Some(chunk) => self.buffer.extend_from_slice(&chunk?),
                None => self.ended = true,
            }
        }
        Ok(())
    }

    /// Takes the header of `binding`'s key.
    pub(super) async fn header(&mut self, binding: Binding<'_>) -> io::Result<()> {
        self.fill(2 + 255).await?;
        let len = binding.header_len(&self.buffer).map_err(damaged)?;
        self.buffer.drain(..len);
        Ok(())
    }

    /// The next stored part and whether it is the last one: it is when no byte follows it.
    pub(super) async fn next(&mut self) -> io::Result<Option<(Vec<u8>, bool)>> {
        self.fill(RECORD + 1).await?;
        if self.buffer.is_empty() {
            return Ok(None);
        }
        let take = self.buffer.len().min(RECORD);
        let record: Vec<u8> = self.buffer.drain(..take).collect();
        Ok(Some((record, self.buffer.is_empty() && self.ended)))
    }
}

/// An I/O error that says the piece does not open, with no byte of it.
pub(super) fn damaged(error: StreamError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

/// The plain bytes of `sealed`, opened part by part with `key` for the piece `piece`. A part
/// that does not open ends the stream with an error, and a stream with no last part ends with
/// one too, so that a cut piece is never taken for a whole one.
pub(super) fn opened(sealed: SealedStream, key: Arc<DataKey>, piece: String) -> PlainStream {
    struct State {
        records: Records,
        key: Arc<DataKey>,
        piece: String,
        number: u64,
        started: bool,
        done: bool,
    }
    let state = State {
        records: Records::new(sealed),
        key,
        piece,
        number: 0,
        started: false,
        done: false,
    };
    Box::pin(stream::try_unfold(state, |mut state| async move {
        if state.done {
            return Ok(None);
        }
        let binding = Binding {
            key: &state.key,
            piece: &state.piece,
        };
        if !state.started {
            state.records.header(binding).await?;
            state.started = true;
        }
        let Some((record, last)) = state.records.next().await? else {
            return Err(damaged(StreamError::Damaged));
        };
        let plain = binding
            .open_part(state.number, last, &record)
            .map_err(damaged)?;
        state.number += 1;
        state.done = last;
        Ok(Some((Bytes::from(plain.to_vec()), state)))
    }))
}

/// Seals plain bytes into parts as they come: a full part is sealed only once a byte after it
/// arrives, so the last part is always marked, also when it is full.
pub(super) struct Sealer {
    part: Zeroizing<Vec<u8>>,
    number: u64,
}

impl Sealer {
    pub(super) fn new() -> Self {
        Self {
            part: Zeroizing::new(Vec::with_capacity(PART)),
            number: 0,
        }
    }

    /// Takes `plain` and gives the stored bytes of each part that is now complete.
    pub(super) fn push(&mut self, binding: Binding<'_>, mut plain: &[u8]) -> Vec<u8> {
        let mut sealed = Vec::new();
        while !plain.is_empty() {
            if self.part.len() == PART {
                sealed.extend(binding.seal_part(self.number, false, &self.part));
                self.part.clear();
                self.number += 1;
            }
            let take = plain.len().min(PART - self.part.len());
            self.part.extend_from_slice(&plain[..take]);
            plain = &plain[take..];
        }
        sealed
    }

    /// The stored bytes of the last part.
    pub(super) fn finish(self, binding: Binding<'_>) -> Vec<u8> {
        binding.seal_part(self.number, true, &self.part)
    }
}
