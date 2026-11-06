//! The running terminal (K4 Sections 8.4 to 8.6): relays bytes both ways and the client's
//! `resize` as a window change until the first cause.
//!
//! One `select!` over the end signal, the client socket, the one write in flight to the
//! device, the device's channel and three timers; each branch is cancel safe, sends to the
//! client happen in the branch body under `writes`' limits, and no lock is held.
//!
//! Backpressure (K4 Section 8.5): the next channel message is read only after the previous
//! frame was sent, and the channel's buffer is bounded (`opening`), so a client that reads
//! slowly stalls the device's channel. The next client message is read only after the
//! previous one was written to the channel, under its window; the time a write waits on the
//! device is not the client's silence, but a write that waits past the request wait ends
//! the session `GATEWAY_ERROR`; the hub's pings go on meanwhile.

use std::{pin::Pin, sync::Arc};

use axum::extract::ws::Message;
use futures_util::FutureExt as _;
use russh::{ChannelMsg, client::Handle};
use tokio::time::{Instant, sleep_until, timeout};

use super::{
    control::{self, Control, Size},
    ending::{DEVICE_STOPPED_READING, ended},
    link::Link,
};
use crate::sessions::{
    EndCause,
    end::{CLOSE_WAIT, Cause, End, unreadable},
    ssh::Observer,
};

/// One client message being written to the device's channel.
pub(super) type Writing = Pin<Box<dyn Future<Output = Result<(), russh::Error>> + Send>>;

/// One step of the loop: whichever came first.
enum Turn {
    Signalled(EndCause),
    Client(Option<Result<Message, axum::Error>>),
    Written(Result<(), russh::Error>),
    Device(Option<ChannelMsg>),
    HubSilent,
    ClientIdle,
    WriteStalled,
}

impl Link<'_> {
    /// Relays until the first cause. `handle` tells, when the channel ends with the
    /// connection, whether the device left or the connection failed.
    pub(super) async fn relay(&mut self, handle: &mut Handle<Observer>) -> End {
        self.client_quiet = Instant::now() + self.timing.client_idle;
        loop {
            let turn = self.next_turn().await;
            if let Some(end) = self.on_turn(turn, handle).await {
                return end;
            }
        }
    }

    /// Waits for whichever comes first. Every branch is cancel safe: the write in flight
    /// stays in `writing` when another branch wins.
    async fn next_turn(&mut self) -> Turn {
        let writing = self.writing.is_some();
        tokio::select! {
            signal = self.tunnel.ended() => Turn::Signalled(signal),
            message = self.client.recv(), if !writing => Turn::Client(message),
            written = written(&mut self.writing), if writing => Turn::Written(written),
            message = self.reader.wait() => Turn::Device(message),
            () = sleep_until(self.hub_quiet) => Turn::HubSilent,
            () = sleep_until(self.client_quiet), if !writing => Turn::ClientIdle,
            () = sleep_until(self.write_due), if writing => Turn::WriteStalled,
        }
    }

    /// Acts on one turn; `Some` when the session ends. Any client message restarts the
    /// client's idle wait, a WebSocket ping or pong included; so does the end of a write the
    /// device held back. The hub's silence restarts with each send (`Link::send`).
    async fn on_turn(&mut self, turn: Turn, handle: &mut Handle<Observer>) -> Option<End> {
        match turn {
            Turn::Signalled(signal) => Some(End::from(signal)),
            Turn::Client(message) => self.on_client(message),
            Turn::Written(written) => {
                self.writing = None;
                self.client_quiet = Instant::now() + self.timing.client_idle;
                written.err().map(|_| ended(Cause::GatewayError))
            }
            Turn::Device(Some(ChannelMsg::Close)) => Some(ended(Cause::RemoteClosed)),
            Turn::Device(Some(message)) => self.output(message).await.err(),
            Turn::Device(None) => Some(gone(handle).await),
            Turn::HubSilent => self.ping().await.err(),
            Turn::ClientIdle => self.client_idle(),
            Turn::WriteStalled => Some(End::explained(Cause::GatewayError, DEVICE_STOPPED_READING)),
        }
    }

    /// One WebSocket message from the client: binary is what the operator typed, text is a
    /// K4 Section 8.4 message; anything else that is not a ping, pong or close breaks the
    /// protocol.
    fn on_client(&mut self, message: Option<Result<Message, axum::Error>>) -> Option<End> {
        self.client_quiet = Instant::now() + self.timing.client_idle;
        let text = match message {
            Some(Ok(Message::Binary(bytes))) => {
                let writer = Arc::clone(&self.writer);
                return self.write(Box::pin(async move { writer.data_bytes(bytes).await }));
            }
            Some(Ok(Message::Text(text))) => text,
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => return None,
            Some(Ok(Message::Close(_))) | None => return Some(ended(Cause::OperatorClosed)),
            Some(Err(error)) => return Some(ended(unreadable(error))),
        };
        match control::parse(&text) {
            Ok(Control::Resize(size)) => {
                let writer = Arc::clone(&self.writer);
                let Size { cols, rows } = size;
                let change = async move {
                    let (cols, rows) = (u32::from(cols), u32::from(rows));
                    writer.window_change(cols, rows, 0, 0).await
                };
                self.write(Box::pin(change))
            }
            Ok(Control::Ping) => None,
            Err(_) => Some(ended(Cause::InvalidInstruction)),
        }
    }

    /// Starts `write` as the one write in flight; the client is not read until it is done.
    fn write(&mut self, write: Writing) -> Option<End> {
        self.writing = Some(write);
        self.write_due = Instant::now() + self.timing.request;
        None
    }

    /// The client's idle wait ran out. A message that arrived while the hub was sending to
    /// the client is read first, so time spent held back by the client is not its silence.
    fn client_idle(&mut self) -> Option<End> {
        match self.client.recv().now_or_never() {
            Some(message) => self.on_client(message),
            None => Some(ended(Cause::Timeout)),
        }
    }
}

/// The write in flight, or never when there is none.
async fn written(writing: &mut Option<Writing>) -> Result<(), russh::Error> {
    match writing {
        Some(write) => write.as_mut().await,
        None => std::future::pending().await,
    }
}

/// The channel ended with its connection: the device leaving with an SSH disconnect is the
/// remote closing; a transport or protocol failure, or no answer within [`CLOSE_WAIT`], is a
/// failed connection (K4 Section 8.6).
async fn gone(handle: &mut Handle<Observer>) -> End {
    match timeout(CLOSE_WAIT, handle).await {
        Ok(Ok(())) => ended(Cause::RemoteClosed),
        Ok(Err(_)) | Err(_) => ended(Cause::GatewayError),
    }
}
