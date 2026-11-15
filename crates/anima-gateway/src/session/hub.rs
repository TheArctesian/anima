//! The connection to the hub: Guacamole instructions in and out.
//!
//! A task of its own reads and decodes, so the session can wait for the next instruction
//! and for other things at once without losing bytes. I/O edge.

use guac_protocol::{Decoder, Instruction, MAX_INSTRUCTION_BYTES, encode};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, WriteHalf};
use tokio::sync::mpsc;

/// Instructions decoded and not yet handled. Small, so a hub that sends faster than the
/// session works is held back and not buffered.
const WAITING_INSTRUCTIONS: usize = 64;

pub(super) struct Hub<S> {
    write: WriteHalf<S>,
    read: mpsc::Receiver<Instruction>,
}

impl<S> Hub<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub(super) fn new(stream: S) -> Self {
        let (from_hub, write) = tokio::io::split(stream);
        let (sender, read) = mpsc::channel(WAITING_INSTRUCTIONS);
        tokio::spawn(decode(from_hub, sender));
        Self { write, read }
    }

    /// The next instruction; `None` when the hub is gone. Safe to drop while waiting.
    pub(super) async fn next(&mut self) -> Option<Instruction> {
        self.read.recv().await
    }

    /// Sends `instructions` in order, as one write.
    pub(super) async fn send(&mut self, instructions: &[Instruction]) -> std::io::Result<()> {
        let wire: String = instructions.iter().map(encode).collect();
        self.write.write_all(wire.as_bytes()).await
    }

    /// Sends bytes that are Guacamole instructions already.
    pub(super) async fn send_wire(&mut self, wire: &[u8]) -> std::io::Result<()> {
        self.write.write_all(wire).await
    }
}

/// Reads and decodes until the hub closes, sends bytes that do not decode, or the session
/// is gone; the channel then closes.
async fn decode<R: AsyncRead + Unpin>(mut from_hub: R, sender: mpsc::Sender<Instruction>) {
    let mut decoder = Decoder::new();
    let mut buffer = vec![0; MAX_INSTRUCTION_BYTES];
    while let Ok(count @ 1..) = from_hub.read(&mut buffer).await {
        let Ok(instructions) = decoder.feed(&buffer[..count]) else {
            return;
        };
        for instruction in instructions {
            if sender.send(instruction).await.is_err() {
                return;
            }
        }
    }
}
