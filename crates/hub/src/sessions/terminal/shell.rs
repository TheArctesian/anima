//! The upgraded terminal up to a running shell (K4 Sections 8.4 and 8.5): the client's first
//! message must be its size, within the first-size wait; the hub then asks the device for a
//! PTY (`xterm-256color`, that size, no pixel size) and the login shell, and hands over to
//! the relay (`relay`). Then it closes both sides.
//!
//! The client sets no other PTY value: the terminal type, the command and the environment
//! are the hub's (GW-013).

use std::sync::Arc;

use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, client::Msg};
use tokio::time::{Instant, sleep_until, timeout_at};

use super::{
    control::{self, Control, Size},
    ending::{NO_SIZE, PTY_REFUSED, SHELL_REFUSED, ended},
    opening::Shell,
    relay::Writing,
    route::Timing,
    writes::{self, close_client, leave_device},
};
use crate::sessions::{
    Tunnel,
    end::{Cause, End, unreadable},
};

/// The terminal type the hub asks the device for (K4 Section 8.4).
const TERM: &str = "xterm-256color";

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
    pub(super) hub_quiet: Instant,
    /// When the client's silence ends the session (`relay`).
    pub(super) client_quiet: Instant,
    /// The client message being written to the device, at most one (`relay`).
    pub(super) writing: Option<Writing>,
    /// When that write has waited on the device too long (`relay`).
    pub(super) write_due: Instant,
}

/// Runs the terminal from the upgrade to its first cause, then closes the client and the
/// device side by side.
pub(super) async fn run(
    client: WebSocket,
    shell: Shell,
    tunnel: &mut Tunnel,
    timing: Timing,
) -> End {
    let Shell {
        mut handle,
        channel,
    } = shell;
    let (reader, writer) = channel.split();
    let now = Instant::now();
    let mut link = Link {
        client,
        reader,
        writer: Arc::new(writer),
        tunnel,
        timing,
        hub_quiet: now + timing.hub_silence,
        client_quiet: now + timing.client_idle,
        writing: None,
        write_due: now,
    };
    let end = match link.start().await {
        Ok(()) => link.relay(&mut handle).await,
        Err(end) => end,
    };
    let Link {
        client,
        writer,
        writing,
        ..
    } = link;
    // A write still waiting on the device is given up.
    drop(writing);
    tokio::join!(
        close_client(client, end.cause),
        leave_device(handle, &writer)
    );
    end
}

impl Link<'_> {
    /// Waits for the client's size, then opens the PTY and the shell at it.
    async fn start(&mut self) -> Result<(), End> {
        let size = self.first_size().await?;
        let cols = u32::from(size.cols);
        let rows = u32::from(size.rows);
        let asked = self
            .writer
            .request_pty(true, TERM, cols, rows, 0, 0, &[])
            .await;
        asked.map_err(|_| ended(Cause::GatewayError))?;
        self.replied(PTY_REFUSED).await?;
        let asked = self.writer.request_shell(true).await;
        asked.map_err(|_| ended(Cause::GatewayError))?;
        self.replied(SHELL_REFUSED).await
    }

    /// The client's first message, which must be `resize` (K4 Section 8.4), within the
    /// first-size wait. WebSocket pings and pongs are not messages and do not count.
    async fn first_size(&mut self) -> Result<Size, End> {
        let due = Instant::now() + self.timing.first_size;
        loop {
            tokio::select! {
                signal = self.tunnel.ended() => return Err(End::from(signal)),
                message = self.client.recv() => match message {
                    Some(Ok(Message::Text(text))) => return first(&text),
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => return Err(ended(Cause::InvalidInstruction)),
                    Some(Ok(Message::Close(_))) | None => return Err(ended(Cause::OperatorClosed)),
                    Some(Err(error)) => return Err(ended(unreadable(error))),
                },
                () = sleep_until(self.hub_quiet) => self.ping().await?,
                () = sleep_until(due) => return Err(End::explained(Cause::Timeout, NO_SIZE)),
            }
        }
    }

    /// The device's answer to the PTY or shell request just sent, within the idle wait:
    /// `Failure` ends the session `GATEWAY_ERROR` with `refused`. Output that comes first goes
    /// to the client; the end of the channel or connection is the remote closing.
    async fn replied(&mut self, refused: &'static str) -> Result<(), End> {
        let due = Instant::now() + self.timing.client_idle;
        loop {
            let message = tokio::select! {
                signal = self.tunnel.ended() => return Err(End::from(signal)),
                message = timeout_at(due, self.reader.wait()) => message.ok(),
            };
            match message {
                Some(Some(ChannelMsg::Success)) => return Ok(()),
                Some(Some(ChannelMsg::Failure)) | None => {
                    return Err(End::explained(Cause::GatewayError, refused));
                }
                Some(Some(ChannelMsg::Close) | None) => return Err(ended(Cause::RemoteClosed)),
                Some(Some(message)) => self.output(message).await?,
            }
        }
    }

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

    /// Sends one message to the client (`writes::send`); the hub's silence restarts once it
    /// is sent.
    pub(super) async fn send(&mut self, message: Message) -> Result<(), End> {
        let wait = self.timing.client_idle;
        writes::send(&mut self.client, message, self.tunnel, wait).await?;
        self.hub_quiet = Instant::now() + self.timing.hub_silence;
        Ok(())
    }
}

/// The first message's meaning: its size, or a protocol error for anything else.
fn first(text: &str) -> Result<Size, End> {
    match control::parse(text) {
        Ok(Control::Resize(size)) => Ok(size),
        Ok(Control::Ping) | Err(_) => Err(ended(Cause::InvalidInstruction)),
    }
}
