//! The terminal's own closing writes: the client's `end` before the shared close
//! (`upgrade::close`), and the hub's leaving of the device. The sends to the client are the
//! shared `upgrade::send`. Each has a short wait of its own.

use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use russh::{
    ChannelWriteHalf, Disconnect,
    client::{Handle, Msg},
};
use tokio::time::timeout;

use super::{control, opening::SignedIn};
use crate::fleet::sessions::{
    end::{CLOSE_WAIT, Cause},
    upgrade::close,
};
use crate::fleet::ssh::Observer;

/// Sends `end` with the session's reason, then closes with the cause's code (K4 Section 8.6,
/// `upgrade::close`). Best effort within [`CLOSE_WAIT`]: the client may be gone, and the
/// session ends either way.
pub(super) async fn close_client(client: WebSocket, cause: Cause) {
    let end = Message::Text(Utf8Bytes::from(control::end(cause.ending().reason)));
    close(client, Some(end), cause).await;
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
