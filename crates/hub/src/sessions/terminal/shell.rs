//! The upgraded terminal up to a running shell (K4 Sections 8.4 and 8.5): the client's first
//! message must be its size, within the first-size wait; the hub then asks the device for a
//! PTY (`xterm-256color`, that size, no pixel size) and the login shell, and hands over to
//! the relay (`relay`). Then it closes both sides.
//!
//! The client sets no other PTY value: the terminal type, the command and the environment
//! are the hub's (GW-013).

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use russh::ChannelMsg;
use tokio::time::{Instant, sleep_until, timeout_at};

use super::{
    control::{self, Control, Size},
    ending::{NO_ANSWER, NO_SIZE, PTY_REFUSED, SHELL_REFUSED, ended},
    link::Link,
    opening::SignedIn,
    route::Timing,
    writes::{close_client, leave_device},
};
use crate::sessions::{
    Tunnel,
    end::{Cause, End, unreadable},
};

/// The terminal type the hub asks the device for (K4 Section 8.4).
const TERM: &str = "xterm-256color";

/// Runs the terminal from the upgrade to its first cause, then closes the client and the
/// device side by side.
pub(super) async fn run(
    client: WebSocket,
    signed_in: SignedIn,
    tunnel: &mut Tunnel,
    timing: Timing,
) -> End {
    let SignedIn {
        mut handle,
        channel,
    } = signed_in;
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
    /// Waits for the client's size, then opens the PTY and the shell at it. Each request
    /// and its answer have the request wait, and race the end signal.
    async fn start(&mut self) -> Result<(), End> {
        let size = self.first_size().await?;
        let (cols, rows) = (u32::from(size.cols), u32::from(size.rows));
        let writer = Arc::clone(&self.writer);
        let pty = async move { writer.request_pty(true, TERM, cols, rows, 0, 0, &[]).await };
        let due = Instant::now() + self.timing.request;
        handed(self.tunnel, pty, due).await?;
        self.replied(PTY_REFUSED, due).await?;
        let writer = Arc::clone(&self.writer);
        let shell = async move { writer.request_shell(true).await };
        let due = Instant::now() + self.timing.request;
        handed(self.tunnel, shell, due).await?;
        self.replied(SHELL_REFUSED, due).await
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

    /// The device's answer to the PTY or shell request just sent, by `due`: `Failure` ends
    /// the session `GATEWAY_ERROR` with `refused`, no answer `GATEWAY_ERROR` as well. Output
    /// that comes first goes to the client; the device closing the channel is the remote
    /// closing, the connection ending under it a failed connection. The hub keeps pinging.
    async fn replied(&mut self, refused: &'static str, due: Instant) -> Result<(), End> {
        loop {
            let message = tokio::select! {
                signal = self.tunnel.ended() => return Err(End::from(signal)),
                message = timeout_at(due, self.reader.wait()) => message.ok(),
                () = sleep_until(self.hub_quiet) => {
                    self.ping().await?;
                    continue;
                }
            };
            match message {
                Some(Some(ChannelMsg::Success)) => return Ok(()),
                Some(Some(ChannelMsg::Failure)) => {
                    return Err(End::explained(Cause::GatewayError, refused));
                }
                None => return Err(End::explained(Cause::GatewayError, NO_ANSWER)),
                Some(Some(ChannelMsg::Close)) => return Err(ended(Cause::RemoteClosed)),
                Some(None) => return Err(ended(Cause::GatewayError)),
                Some(Some(message)) => self.output(message).await?,
            }
        }
    }
}

/// Hands one request to the connection by `due`, unless the session ends first; a request
/// still waiting at `due` ends the session `GATEWAY_ERROR`. The hand-off is to russh's queue
/// of a few commands, of which the hub uses one at a time, so it waits only when the
/// connection's task has stopped: there is no ping here, unlike the wait for the answer.
/// No ping during the hand-off is by design, as K4 Section 8.4 says: it waits only if the
/// connection's task stopped, and the request wait ends it.
pub(super) async fn handed(
    tunnel: &mut Tunnel,
    request: impl Future<Output = Result<(), russh::Error>>,
    due: Instant,
) -> Result<(), End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        handed = timeout_at(due, request) => match handed {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(ended(Cause::GatewayError)),
            Err(_) => Err(End::explained(Cause::GatewayError, NO_ANSWER)),
        },
    }
}

/// The first message's meaning: its size, or a protocol error for anything else.
fn first(text: &str) -> Result<Size, End> {
    match control::parse(text) {
        Ok(Control::Resize(size)) => Ok(size),
        Ok(Control::Ping) | Err(_) => Err(ended(Cause::InvalidInstruction)),
    }
}
