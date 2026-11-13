//! One request at a time over the files tunnel (K4 Section 9): a listing, a download of
//! exactly the announced size, or an upload of exactly the declared size. A `ping` goes out
//! after 5 quiet seconds; 15 silent seconds lose the tunnel. Byte counts from the hub are
//! checked, never trusted: a frame past the size, or `done` before it, stops the transfer.

use std::io::{self, Read, Write};

use futures_util::FutureExt as _;
use tokio::time::{Instant, sleep_until};

use super::codec::{self, HubMessage, Listing, Refusal, Request, UPLOAD_FRAME};
use crate::hub::tunnel::{Frame, Socket, TunnelError};
use crate::sessions::pace::{Due, Pace};

/// How a files request failed.
#[derive(Debug, thiserror::Error)]
pub enum FilesError {
    #[error("the hub refused the request ({0:?})")]
    Refused(Refusal),
    #[error("the request cannot be sent: the path is empty, holds NUL or is too long")]
    Unsendable,
    #[error("the hub sent a files message this program does not accept")]
    Protocol,
    #[error("the files tunnel closed")]
    Closed,
    #[error("nothing came from the hub for 15 seconds")]
    Lost,
    #[error(transparent)]
    Tunnel(#[from] TunnelError),
    #[error("{0}")]
    Local(io::Error),
}

/// What came from the hub, pings taken out.
enum Incoming {
    Message(HubMessage),
    Bytes(Vec<u8>),
}

/// An open files tunnel.
pub struct Files {
    socket: Socket,
    pace: Pace,
}

impl Files {
    #[must_use]
    pub fn new(socket: Socket) -> Self {
        Self {
            socket,
            pace: Pace::new(Instant::now()),
        }
    }

    /// Closes the tunnel; best effort.
    pub async fn close(self) {
        self.socket.close().await;
    }

    /// The entries of the directory `path`.
    ///
    /// # Errors
    /// [`FilesError`]; `Refused` as the device's answer.
    pub async fn list(&mut self, path: &str) -> Result<Listing, FilesError> {
        self.request(&Request::List { path }).await?;
        match self.next().await? {
            Incoming::Message(HubMessage::Entries(listing)) => Ok(listing),
            Incoming::Message(HubMessage::Refused(reason)) => Err(FilesError::Refused(reason)),
            _ => Err(FilesError::Protocol),
        }
    }

    /// The file at `path`, written to `sink`; returns its size.
    ///
    /// # Errors
    /// As [`Files::list`]; `Local` when `sink` fails.
    pub async fn download(&mut self, path: &str, sink: &mut dyn Write) -> Result<u64, FilesError> {
        self.request(&Request::Download { path }).await?;
        let size = match self.next().await? {
            Incoming::Message(HubMessage::Download { size }) => size,
            Incoming::Message(HubMessage::Refused(reason)) => {
                return Err(FilesError::Refused(reason));
            }
            _ => return Err(FilesError::Protocol),
        };
        let mut received: u64 = 0;
        loop {
            match self.next().await? {
                Incoming::Bytes(bytes) => {
                    received = u64::try_from(bytes.len())
                        .ok()
                        .and_then(|length| received.checked_add(length))
                        .filter(|total| *total <= size)
                        .ok_or(FilesError::Protocol)?;
                    sink.write_all(&bytes).map_err(FilesError::Local)?;
                }
                Incoming::Message(HubMessage::Done) if received == size => return Ok(size),
                Incoming::Message(_) => return Err(FilesError::Protocol),
            }
        }
    }

    /// Uploads `size` bytes from `source` to `path`, which must not exist yet.
    ///
    /// # Errors
    /// As [`Files::download`]; `Local` when `source` ends early or fails.
    pub async fn upload(
        &mut self,
        path: &str,
        source: &mut dyn Read,
        size: u64,
    ) -> Result<(), FilesError> {
        self.request(&Request::Upload { path, size }).await?;
        let mut left = size;
        let mut buffer = vec![0_u8; UPLOAD_FRAME];
        while left > 0 {
            if let Some(early) = self.socket.recv().now_or_never() {
                self.early(early)?;
            }
            let want = usize::try_from(left).map_or(UPLOAD_FRAME, |left| left.min(UPLOAD_FRAME));
            source
                .read_exact(&mut buffer[..want])
                .map_err(FilesError::Local)?;
            self.socket.send_binary(buffer[..want].to_vec()).await?;
            self.pace.sent(Instant::now());
            left -= want as u64;
        }
        match self.next().await? {
            Incoming::Message(HubMessage::Done) => Ok(()),
            Incoming::Message(HubMessage::Refused(reason)) => Err(FilesError::Refused(reason)),
            _ => Err(FilesError::Protocol),
        }
    }

    /// A message that came while an upload was sending: a refusal stops it; a ping or a
    /// WebSocket control frame is only a sign of life; anything else is out of turn.
    fn early(&mut self, frame: Result<Frame, TunnelError>) -> Result<(), FilesError> {
        self.pace.heard(Instant::now());
        match frame? {
            Frame::Alive => Ok(()),
            Frame::Text(text) => match codec::decode(&text) {
                Some(HubMessage::Ping) => Ok(()),
                Some(HubMessage::Refused(reason)) => Err(FilesError::Refused(reason)),
                _ => Err(FilesError::Protocol),
            },
            Frame::Closed(_) => Err(FilesError::Closed),
            Frame::Binary(_) => Err(FilesError::Protocol),
        }
    }

    async fn request(&mut self, request: &Request<'_>) -> Result<(), FilesError> {
        let text = codec::encode(request).ok_or(FilesError::Unsendable)?;
        self.socket.send_text(&text).await?;
        self.pace.sent(Instant::now());
        Ok(())
    }

    /// The next message or bytes from the hub, answering the pace meanwhile.
    async fn next(&mut self) -> Result<Incoming, FilesError> {
        loop {
            tokio::select! {
                frame = self.socket.recv() => {
                    self.pace.heard(Instant::now());
                    match frame? {
                        Frame::Alive => {}
                        Frame::Binary(bytes) => return Ok(Incoming::Bytes(bytes)),
                        Frame::Text(text) => match codec::decode(&text) {
                            Some(HubMessage::Ping) => {}
                            Some(message) => return Ok(Incoming::Message(message)),
                            None => return Err(FilesError::Protocol),
                        },
                        Frame::Closed(_) => return Err(FilesError::Closed),
                    }
                }
                () = sleep_until(self.pace.next()) => match self.pace.due(Instant::now()) {
                    Some(Due::Ping) => {
                        self.socket.send_text(codec::PING).await?;
                        self.pace.sent(Instant::now());
                    }
                    Some(Due::Lost) => return Err(FilesError::Lost),
                    None => {}
                },
            }
        }
    }
}
