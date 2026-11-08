//! One upgraded terminal (K4 Section 8.4): the client socket, the device's channel and the
//! deadlines, and the hub's sends to the client that the shell's start and the relay share.

use std::sync::Arc;

use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, client::Msg};
use tokio::time::Instant;

use super::{control, relay::Writing, route::Timing};
use crate::sessions::{Tunnel, end::End, quiet::Quiet, upgrade};

/// One upgraded terminal: the client socket, the device's channel, the end signal, the waits
/// and the deadlines they set.
pub(super) struct Link<'a> {
    pub(super) client: WebSocket,
    pub(super) reader: ChannelReadHalf,
    /// Shared with the one write in flight to the device (`relay`).
    pub(super) writer: Arc<ChannelWriteHalf<Msg>>,
    pub(super) tunnel: &'a mut Tunnel,
    pub(super) timing: Timing,
    /// When the hub's silence earns the client a `ping`.
    pub(super) hub_quiet: Quiet,
    /// When the client's silence ends the session (`relay`).
    pub(super) client_quiet: Quiet,
    /// The client message being written to the device, at most one (`relay`).
    pub(super) writing: Option<Writing>,
    /// When that write has waited on the device too long (`relay`).
    pub(super) write_due: Instant,
}

impl Link<'_> {
    /// Relays one channel message to the client: standard output and standard error (`ext`
    /// 1) as binary frames, unchanged and not kept (RA-008); anything else is dropped here.
    pub(super) async fn output(&mut self, message: ChannelMsg) -> Result<(), End> {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, ext: 1 } => {
                self.send(Message::Binary(data)).await
            }
            _ => Ok(()),
        }
    }

    /// Sends the hub's `ping` (K4 Section 8.4).
    pub(super) async fn ping(&mut self) -> Result<(), End> {
        self.send(Message::Text(Utf8Bytes::from_static(control::PING)))
            .await
    }

    /// Sends one message to the client (`upgrade::send`); the hub's silence restarts once it
    /// is sent.
    pub(super) async fn send(&mut self, message: Message) -> Result<(), End> {
        let wait = self.timing.client_idle;
        upgrade::send(&mut self.client, message, self.tunnel, wait).await?;
        self.hub_quiet.restart(Instant::now());
        Ok(())
    }
}
