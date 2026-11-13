//! One command on a link (D-3, CHK-012): its own session channel, one `exec` and no other
//! request, standard output up to the cap, standard error dropped unread, the exit status.
//! I/O edge; nothing here logs or keeps the output past the call.

use std::time::Duration;

use russh::{
    Channel, ChannelMsg,
    client::{Handle, Msg},
};
use tokio::time::timeout;

use super::{CommandOutput, Error, OUTPUT_CAP};
use crate::fleet::ssh::Observer;

/// The longest the hub waits to hand the channel's close to the connection, past the
/// command's limit; a connection that cannot take it within this is stalled.
const CLOSE_WAIT: Duration = Duration::from_secs(1);

/// Runs `line` on a new session channel of `handle`, all within `limit`, then closes the
/// channel within [`CLOSE_WAIT`], so the device ends the command and the link is ready for
/// the next one.
///
/// # Errors
/// [`Error::TimeLimit`], [`Error::CommandRefused`] or [`Error::Lost`].
pub(super) async fn run(
    handle: &Handle<Observer>,
    line: &str,
    limit: Duration,
) -> Result<CommandOutput, Error> {
    let mut opened = None;
    let ran = timeout(limit, async {
        let channel = opened.insert(open(handle).await?);
        exchange(channel, line).await
    })
    .await
    .map_err(|_| Error::TimeLimit);
    if let Some(channel) = opened {
        // A close that fails or stalls means the connection is gone or stuck; the next
        // command reports that, and this command's result stands.
        let _closed = timeout(CLOSE_WAIT, channel.close()).await;
    }
    ran?
}

/// Opens the command's session channel.
async fn open(handle: &Handle<Observer>) -> Result<Channel<Msg>, Error> {
    handle
        .channel_open_session()
        .await
        .map_err(|error| match error {
            russh::Error::ChannelOpenFailure(_) => Error::CommandRefused,
            _ => Error::Lost,
        })
}

/// Sends the one `exec` and reads the channel until the device closes it, or until the output
/// reaches [`OUTPUT_CAP`].
async fn exchange(channel: &mut Channel<Msg>, line: &str) -> Result<CommandOutput, Error> {
    channel.exec(true, line).await.map_err(|_| Error::Lost)?;
    let mut output = CommandOutput {
        exit_status: None,
        stdout: Vec::new(),
        cut: false,
    };
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Failure => return Err(Error::CommandRefused),
            ChannelMsg::Data { data } => {
                if !kept(&mut output.stdout, &data) {
                    output.cut = true;
                    output.exit_status = None;
                    return Ok(output);
                }
            }
            ChannelMsg::ExitStatus { exit_status } => output.exit_status = Some(exit_status),
            ChannelMsg::Close => return Ok(output),
            // Standard error, the exec's success reply, EOF, window changes, exit signals:
            // nothing the caller reads.
            _ => {}
        }
    }
    // The channel ended without the device closing it: the connection went away.
    Err(Error::Lost)
}

/// Adds `data` to `stdout` up to [`OUTPUT_CAP`] bytes; `false` when some of it did not fit.
pub(super) fn kept(stdout: &mut Vec<u8>, data: &[u8]) -> bool {
    let room = OUTPUT_CAP.saturating_sub(stdout.len());
    let fits = data.len() <= room;
    stdout.extend_from_slice(&data[..data.len().min(room)]);
    fits
}
