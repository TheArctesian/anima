//! The hub's writes to the client and its leaving of the device. Every send to the client
//! has a time limit and races the end signal, so a client that stops reading cannot hide an
//! `endSession` or a shutdown: the first cause wins (D-9). The writes that close the session
//! have a short wait of their own.

use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::SinkExt;
use russh::{
    ChannelWriteHalf, Disconnect,
    client::{Handle, Msg},
};
use tokio::time::timeout;

use super::{control, opening::SignedIn};
use crate::sessions::{
    Tunnel,
    end::{CLIENT_STOPPED_READING, CLOSE_WAIT, Cause, End},
    ssh::Observer,
};

/// Sends one message to the client, unless the session ends first. A failed send is the
/// client leaving; one that stalls past `wait` is a timeout (K4 Section 8.5).
pub(super) async fn send(
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
            Err(_) => Err(End::explained(Cause::Timeout, CLIENT_STOPPED_READING)),
        },
    }
}

/// Sends `end` with the session's reason, then the close frame with the cause's code (K4
/// Section 8.6), then flushes and closes the socket. Best effort within [`CLOSE_WAIT`]: the
/// client may be gone, and the session ends either way. When the client closed first, the
/// WebSocket library refuses both sends, and its echo of the client's close is what goes out
/// (RFC 6455 Section 5.5.1).
pub(super) async fn close_client(mut client: WebSocket, cause: Cause) {
    let ending = cause.ending();
    let end = Message::Text(Utf8Bytes::from(control::end(ending.reason)));
    let frame = CloseFrame {
        code: ending.close_code,
        reason: Utf8Bytes::default(),
    };
    let _ = timeout(CLOSE_WAIT, async {
        let _ = client.send(end).await;
        let _ = client.send(Message::Close(Some(frame))).await;
        SinkExt::close(&mut client).await
    })
    .await;
}

/// Closes the SSH channel, then the connection. Best effort within [`CLOSE_WAIT`]: the
/// device may be gone, and dropping the handle ends the connection either way.
pub(super) async fn leave_device(handle: Handle<Observer>, channel: &ChannelWriteHalf<Msg>) {
    let _ = timeout(CLOSE_WAIT, async {
        let _ = channel.close().await;
        handle.disconnect(Disconnect::ByApplication, "", "").await
    })
    .await;
}

/// Leaves a device whose shell never started.
pub(super) async fn leave_unstarted(signed_in: SignedIn) {
    let SignedIn { handle, channel } = signed_in;
    let (_, writer) = channel.split();
    leave_device(handle, &writer).await;
}
