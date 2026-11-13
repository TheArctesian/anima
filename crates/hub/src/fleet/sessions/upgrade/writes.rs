//! The hub's writes to the client that every WebSocket tunnel makes: one send that races the
//! end signal, so a client that stops reading cannot hide an `endSession` or a shutdown (the
//! first cause wins, D-9), and the close, with a short wait of its own.

use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::SinkExt;
use tokio::time::timeout;

use crate::fleet::sessions::{
    Tunnel,
    end::{CLIENT_STOPPED_READING, CLOSE_WAIT, Cause, End},
};

/// Sends one message to the client, unless the session ends first. A failed send is the
/// client leaving; one that stalls past `wait` is a timeout (K4 Sections 4 and 8.5).
pub(in crate::fleet::sessions) async fn send(
    client: &mut WebSocket,
    message: Message,
    tunnel: &mut Tunnel,
    wait: Duration,
) -> Result<(), End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        sent = timeout(wait, client.send(message)) => match sent {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
            Err(_) => Err(End::worded(Cause::Timeout, CLIENT_STOPPED_READING)),
        },
    }
}

/// Sends `last`, when there is one, then the close frame with the cause's code (K4 Sections 5,
/// 8.6 and 9), then flushes and closes the socket. Best effort within [`CLOSE_WAIT`]: the
/// client may be gone, and the session ends either way. When the client closed first, the
/// WebSocket library refuses the sends, and its echo of the client's close is what goes out
/// (RFC 6455 Section 5.5.1); closing the sink flushes it.
pub(in crate::fleet::sessions) async fn close(
    mut client: WebSocket,
    last: Option<Message>,
    cause: Cause,
) {
    let frame = CloseFrame {
        code: cause.ending().close_code,
        reason: Utf8Bytes::default(),
    };
    let _ = timeout(CLOSE_WAIT, async {
        if let Some(last) = last {
            let _ = client.send(last).await;
        }
        let _ = client.send(Message::Close(Some(frame))).await;
        SinkExt::close(&mut client).await
    })
    .await;
}
