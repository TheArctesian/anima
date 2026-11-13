//! The hub's writes to the client and to guacd. Every write has a time limit. While the
//! tunnel is open each write also races the end signal, so a peer that stops reading cannot
//! hide an `endSession` or a shutdown: the first cause wins and the audit names who ended
//! the session (D-9). The writes that close the tunnel have a short wait of their own; the
//! client's is the shared `upgrade::close`.

use std::time::Duration;

use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use guac_protocol::{Instruction, encode};
use tokio::{io::AsyncWriteExt as _, net::TcpStream, time::timeout};

use super::super::ending::{CLOSE_WAIT, End, gateway_written};
use crate::fleet::sessions::{Tunnel, upgrade::send};

/// What the hub tells guacd when it leaves first.
const DISCONNECT: &[u8] = b"10.disconnect;";

/// Sends one instruction to the client as one text message, unless the session ends first
/// (`upgrade::send`). A failed send is the client leaving; one that stalls past `wait` is a
/// timeout.
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
    send(client, message, tunnel, wait).await
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

/// Tells guacd the hub is leaving, then closes the connection. Best effort within
/// [`CLOSE_WAIT`]: guacd may be gone, and dropping the socket ends its side either way.
pub(super) async fn leave_gateway(mut gateway: TcpStream) {
    let _ = timeout(CLOSE_WAIT, async {
        gateway.write_all(DISCONNECT).await?;
        gateway.shutdown().await
    })
    .await;
}
