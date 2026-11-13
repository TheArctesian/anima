//! The `sftp` subsystem on a link (RA-032, K4 Section 9): one session channel with one
//! `subsystem` request and no other, turned into a byte stream for the SFTP client. I/O edge;
//! nothing here logs.

use std::time::Duration;

use russh::{ChannelMsg, ChannelStream, client::Msg};
use tokio::time::timeout;

use super::{Error, SshLink};

/// The SSH name of the only subsystem the hub asks a device for.
const SFTP: &str = "sftp";

impl SshLink {
    /// Opens a session channel, asks for the `sftp` subsystem on it and waits up to `limit`
    /// for the device's answer. No PTY, shell, `exec`, environment, agent or forwarding
    /// request is sent. The stream ends when the channel closes; the link must outlive it.
    ///
    /// # Errors
    /// [`Error::TimeLimit`] when `limit` passes first, [`Error::CommandRefused`] when the
    /// device refuses the channel or the subsystem, [`Error::Lost`] when the connection fails.
    pub(crate) async fn sftp(&self, limit: Duration) -> Result<ChannelStream<Msg>, Error> {
        timeout(limit, async {
            let mut channel =
                self.handle
                    .channel_open_session()
                    .await
                    .map_err(|error| match error {
                        russh::Error::ChannelOpenFailure(_) => Error::CommandRefused,
                        _ => Error::Lost,
                    })?;
            channel
                .request_subsystem(true, SFTP)
                .await
                .map_err(|_| Error::Lost)?;
            loop {
                match channel.wait().await {
                    Some(ChannelMsg::Success) => return Ok(channel.into_stream()),
                    Some(ChannelMsg::Failure | ChannelMsg::Close | ChannelMsg::Eof) => {
                        return Err(Error::CommandRefused);
                    }
                    // Window adjustments and anything else before the answer.
                    Some(_) => {}
                    None => return Err(Error::Lost),
                }
            }
        })
        .await
        .map_err(|_| Error::TimeLimit)?
    }
}
