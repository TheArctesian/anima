//! The hub's writes to the client and to guacd. Every write has a time limit. While the
//! tunnel is open each write also races the end signal, so a peer that stops reading cannot
//! hide an `endSession` or a shutdown: the first cause wins and the audit names who ended
//! the session (D-9). The writes that close the tunnel have a short wait of their own.

use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::SinkExt;
use guac_protocol::{Instruction, encode};
use tokio::{io::AsyncWriteExt as _, net::TcpStream, time::timeout};

use super::super::ending::{CLOSE_WAIT, Cause, End, gateway_written};
use crate::sessions::Tunnel;

/// What the hub tells guacd when it leaves first.
const DISCONNECT: &[u8] = b"10.disconnect;";

/// The event message when a write to the client stalls.
const CLIENT_STOPPED_READING: &str =
    "The client stopped reading what the hub sent, so the hub closed the SSH session.";

/// Sends one instruction to the client as one text message, unless the session ends first. A
/// failed send is the client leaving; one that stalls past `wait` is a timeout.
///
/// Characters outside the Basic Multilingual Plane go out as U+FFFD, so each length is the
/// same in code points and UTF-16 units and remote text such as a terminal title cannot end
/// an element early in guacamole-common-js before 1.5.2 (K4 Section 4, R-046).
pub(super) async fn send_text(
    client: &mut WebSocket,
    instruction: &Instruction,
    tunnel: &mut Tunnel,
    wait: Duration,
) -> Result<(), End> {
    let message = Message::Text(Utf8Bytes::from(encode(&instruction.with_bmp_text())));
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        sent = timeout(wait, client.send(message)) => match sent {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
            Err(_) => Err(End::explained(Cause::Timeout, CLIENT_STOPPED_READING)),
        },
    }
}

/// Writes `wire` to guacd, unless the session ends first. A write that fails or stalls past
/// `wait` is a gateway error.
pub(super) async fn write_gateway(
    gateway: &mut TcpStream,
    wire: &[u8],
    tunnel: &mut Tunnel,
    wait: Duration,
) -> Result<(), End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        written = timeout(wait, gateway.write_all(wire)) => gateway_written(written.ok().as_ref()),
    }
}

/// Sends the close frame with the cause's code (K4 Section 5), then flushes and closes the
/// socket. When the client closed first, the WebSocket library refuses a second close frame
/// and its echo of the client's frame is what goes out (RFC 6455 Section 5.5.1); closing the
/// sink flushes it. Best effort within [`CLOSE_WAIT`]: the client may be gone, and the
/// session ends either way.
pub(super) async fn close_client(mut client: WebSocket, cause: Cause) {
    let frame = CloseFrame {
        code: cause.ending().close_code,
        reason: Utf8Bytes::default(),
    };
    let _ = timeout(CLOSE_WAIT, async {
        let _ = client.send(Message::Close(Some(frame))).await;
        SinkExt::close(&mut client).await
    })
    .await;
}

/// Tells guacd the hub is leaving, then closes the connection. Best effort within
/// [`CLOSE_WAIT`]: guacd may be gone, and dropping the socket ends its side either way.
pub(super) async fn leave_gateway(mut gateway: TcpStream) {
    let _ = timeout(CLOSE_WAIT, async {
        gateway.write_all(DISCONNECT).await?;
        gateway.shutdown().await
    })
    .await;
}
