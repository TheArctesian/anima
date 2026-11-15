//! The connection to the hub: Guacamole instructions in and out.
//!
//! A task of its own reads and decodes, so the session can wait for the next instruction
//! and for other things at once without losing bytes. I/O edge.

use std::io;
use std::time::Duration;

use guac_protocol::{Decoder, Instruction, MAX_INSTRUCTION_BYTES, encode};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, WriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use zeroize::Zeroize as _;

/// Instructions decoded and not yet handled. Small, so a hub that sends faster than the
/// session works is held back and not buffered.
const WAITING_INSTRUCTIONS: usize = 64;

pub(super) struct Hub<S> {
    write: WriteHalf<S>,
    read: mpsc::Receiver<Instruction>,
    write_wait: Duration,
    reader: JoinHandle<()>,
}

impl<S> Drop for Hub<S> {
    /// The reader holds the connection's other half: it stops with the session.
    fn drop(&mut self) {
        self.reader.abort();
    }
}

impl<S> Hub<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub(super) fn new(stream: S, write_wait: Duration) -> Self {
        let (from_hub, write) = tokio::io::split(stream);
        let (sender, read) = mpsc::channel(WAITING_INSTRUCTIONS);
        let reader = tokio::spawn(decode(from_hub, sender));
        Self {
            write,
            read,
            write_wait,
            reader,
        }
    }

    /// The next instruction; `None` when the hub is gone. Safe to drop while waiting.
    pub(super) async fn next(&mut self) -> Option<Instruction> {
        self.read.recv().await
    }

    /// Sends `instructions` in order, as one write.
    pub(super) async fn send(&mut self, instructions: &[Instruction]) -> io::Result<()> {
        let wire: String = instructions.iter().map(encode).collect();
        self.send_wire(wire.as_bytes()).await
    }

    /// Sends bytes that are Guacamole instructions already. A hub that does not take them
    /// within the write limit has stopped reading.
    pub(super) async fn send_wire(&mut self, wire: &[u8]) -> io::Result<()> {
        match timeout(self.write_wait, self.write.write_all(wire)).await {
            Ok(written) => written,
            Err(_) => Err(io::ErrorKind::TimedOut.into()),
        }
    }
}

/// Reads and decodes until the hub closes, sends bytes that do not decode, or the session
/// is gone; the channel then closes.
async fn decode<R: AsyncRead + Unpin>(mut from_hub: R, sender: mpsc::Sender<Instruction>) {
    let mut decoder = Decoder::new();
    let mut buffer = vec![0; MAX_INSTRUCTION_BYTES];
    while let Ok(count @ 1..) = from_hub.read(&mut buffer).await {
        let fed = decoder.feed(&buffer[..count]);
        // The bytes may be a `connect` or a part of a credential's answer.
        buffer[..count].zeroize();
        let Ok(instructions) = fed else { return };
        for instruction in instructions {
            if sender.send(instruction).await.is_err() {
                return;
            }
        }
    }
}
