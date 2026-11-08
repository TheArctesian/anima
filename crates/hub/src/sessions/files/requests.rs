//! What the files tunnel does with each client message and each device answer (K4 Section 9):
//! one transfer at a time; the device's transfer flags read at each request; an upload takes
//! exactly its size and never replaces a file; each finished or refused transfer is recorded.

use std::sync::Arc;

use axum::{body::Bytes, extract::ws::Message};
use sqlx::PgPool;
use tokio::time::Instant;

use super::{
    device::{Asked, Broken, Handle, Remote},
    protocol::{self, DevicePath, Refusal, Request},
    relay::{Active, Done, Relay, Stage},
    transfer::{Direction, Outcome, Transfer},
};
use crate::{
    devices::{self, TransferPolicy},
    ids::DeviceId,
    sessions::end::{Cause, unreadable},
};

/// The device's transfer flags now, or `None` when the device is gone.
async fn flags(db: &PgPool, device: DeviceId) -> Result<Option<TransferPolicy>, Broken> {
    match devices::get(db, device).await {
        Ok(device) => Ok(Some(device.transfer)),
        Err(devices::Error::NotFound(_)) => Ok(None),
        Err(_) => Err(Broken::Devices),
    }
}

/// Opens `path` for a download when the device's flags allow it at this request.
async fn readable(db: PgPool, device: DeviceId, remote: Arc<Remote>, path: DevicePath) -> Done {
    let open: Asked<(Handle, u64)> = match flags(&db, device).await {
        Ok(Some(flags)) if flags.download => remote.open_download(&path).await,
        Ok(Some(_)) => Ok(Err(Refusal::DownloadOff)),
        Ok(None) => Ok(Err(Refusal::Denied)),
        Err(broken) => Err(broken),
    };
    Done::Readable(open)
}

/// Creates `path` for an upload when the device's flags allow it at this request.
async fn created(db: PgPool, device: DeviceId, remote: Arc<Remote>, path: DevicePath) -> Done {
    let create: Asked<Handle> = match flags(&db, device).await {
        Ok(Some(flags)) if flags.upload => remote.create(&path).await,
        Ok(Some(_)) => Ok(Err(Refusal::UploadOff)),
        Ok(None) => Ok(Err(Refusal::Denied)),
        Err(broken) => Err(broken),
    };
    Done::Created(create)
}

impl Relay {
    /// One WebSocket message from the client. Any message restarts the client's silence.
    pub(super) fn on_client(
        &mut self,
        message: Option<Result<Message, axum::Error>>,
    ) -> Result<(), Cause> {
        self.client_quiet.restart(Instant::now());
        match message {
            Some(Ok(Message::Binary(bytes))) => self.on_frame(bytes),
            Some(Ok(Message::Text(text))) => match protocol::parse(&text) {
                Ok(request) => self.on_request(request),
                Err(_) => Err(Cause::InvalidInstruction),
            },
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => Ok(()),
            Some(Ok(Message::Close(_))) | None => Err(Cause::OperatorClosed),
            Some(Err(error)) => Err(unreadable(error)),
        }
    }

    /// A binary frame: upload bytes, only while an upload takes them, never past its size.
    fn on_frame(&mut self, bytes: Bytes) -> Result<(), Cause> {
        let Some(Active { transfer, stage }) = self.active.as_mut() else {
            return Err(Cause::InvalidInstruction);
        };
        if transfer.direction != Direction::Upload {
            return Err(Cause::InvalidInstruction);
        }
        let offset = transfer
            .take(bytes.len())
            .map_err(|_| Cause::InvalidInstruction)?;
        let complete = transfer.is_complete();
        match stage {
            Stage::Discarding if complete => self.active = None,
            Stage::Open(handle) if !bytes.is_empty() => {
                let (remote, handle) = (Arc::clone(&self.remote), handle.clone());
                self.start(async move {
                    Done::Wrote(remote.write(&handle, offset, bytes.to_vec()).await)
                });
            }
            Stage::Discarding | Stage::Open(_) => {}
            Stage::Opening | Stage::Closing(_) => return Err(Cause::InvalidInstruction),
        }
        Ok(())
    }

    /// A text request. While a transfer is in flight only `cancel` and `ping` are accepted.
    fn on_request(&mut self, request: Request) -> Result<(), Cause> {
        let (db, device) = (self.trail.db.clone(), self.trail.ticket.device);
        let remote = Arc::clone(&self.remote);
        match (request, self.active.is_some()) {
            (Request::Ping, _) | (Request::Cancel, false) => {}
            (Request::Cancel, true) => self.cancel(),
            (_, true) => return Err(Cause::InvalidInstruction),
            (Request::List(path), false) => {
                let cap = self.max_entries;
                self.start(async move { Done::Listed(remote.list(&path, cap).await) });
            }
            (Request::Download(path), false) => {
                self.begin(Transfer::new(Direction::Download, path.clone(), 0));
                self.start(readable(db, device, remote, path));
            }
            (Request::Upload { path, size }, false) => {
                self.begin(Transfer::new(Direction::Upload, path.clone(), size));
                self.start(created(db, device, remote, path));
            }
        }
        Ok(())
    }

    fn begin(&mut self, transfer: Transfer) {
        self.active = Some(Active {
            transfer,
            stage: Stage::Opening,
        });
    }

    /// Ends the transfer in flight at the client's request: an open file is closed (an
    /// upload's is also removed) and `done` follows; a refused upload stops dropping frames.
    fn cancel(&mut self) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        match &active.stage {
            Stage::Open(handle) => {
                let handle = handle.clone();
                let upload = active.transfer.direction == Direction::Upload;
                let path = active.transfer.path.clone();
                active.stage = Stage::Closing(Outcome::Cancelled);
                let remote = Arc::clone(&self.remote);
                self.start(async move {
                    let closed = remote.close(&handle).await;
                    if upload {
                        return Done::Closed(remote.remove(&path).await);
                    }
                    Done::Closed(closed)
                });
            }
            Stage::Discarding => self.active = None,
            Stage::Opening | Stage::Closing(_) => {}
        }
    }

    /// One answer from the device. The time it took is not the client's silence.
    pub(super) async fn on_done(&mut self, done: Done) -> Result<(), Cause> {
        self.work = None;
        self.device_quiet.restart(Instant::now());
        self.client_quiet.restart(Instant::now());
        match done {
            Done::Listed(Ok(Ok(listing))) => self.send_owned(protocol::entries(&listing)).await,
            Done::Listed(Ok(Err(refused))) => self.send_owned(protocol::refused(refused)).await,
            Done::Readable(Ok(Ok((handle, size)))) => self.opened(handle, Some(size)).await,
            Done::Created(Ok(Ok(handle))) => self.opened(handle, None).await,
            Done::Readable(Ok(Err(refused))) | Done::Created(Ok(Err(refused))) => {
                self.refused(refused).await
            }
            Done::Read(Ok(data), asked) => self.read(data, asked).await,
            Done::Wrote(Ok(())) => {
                self.close_if_complete();
                Ok(())
            }
            Done::Closed(closed) => self.closed(closed).await,
            Done::Alive(Ok(())) => Ok(()),
            Done::Listed(Err(broken))
            | Done::Readable(Err(broken))
            | Done::Created(Err(broken))
            | Done::Read(Err(broken), _)
            | Done::Wrote(Err(broken))
            | Done::Alive(Err(broken)) => Err(self.broken(broken)),
        }
    }

    async fn send_owned(&mut self, text: String) -> Result<(), Cause> {
        self.send(Message::Text(text.into())).await
    }

    /// The file opened: a download announces its size and starts reading; an upload waits
    /// for frames. A transfer of 0 bytes closes at once.
    async fn opened(&mut self, handle: Handle, size: Option<u64>) -> Result<(), Cause> {
        let Some(active) = self.active.as_mut() else {
            return Err(Cause::GatewayError);
        };
        active.stage = Stage::Open(handle);
        if let Some(size) = size {
            active.transfer.size = size;
            self.send_owned(protocol::download(size)).await?;
            if size > 0 {
                self.read_next();
                return Ok(());
            }
        }
        self.close_if_complete();
        Ok(())
    }

    /// The request was refused: the audit entry, then `refused`; a refused upload's frames
    /// are dropped up to its size.
    async fn refused(&mut self, reason: Refusal) -> Result<(), Cause> {
        let Some(active) = self.active.take() else {
            return Err(Cause::GatewayError);
        };
        if !self
            .audit(&active.transfer, Outcome::Refused, Some(reason))
            .await
        {
            return Err(Cause::GatewayError);
        }
        self.send_owned(protocol::refused(reason)).await?;
        let transfer = active.transfer;
        if transfer.direction == Direction::Upload && transfer.size > 0 {
            self.active = Some(Active {
                transfer,
                stage: Stage::Discarding,
            });
        }
        Ok(())
    }

    /// A download's chunk: sent as one frame, then the next read, or the close at the end.
    async fn read(&mut self, data: Vec<u8>, asked: u32) -> Result<(), Cause> {
        let Some(active) = self.active.as_mut() else {
            return Err(Cause::GatewayError);
        };
        if active.transfer.add_read(data.len(), asked).is_err() {
            return Err(self.broken(Broken::Sftp));
        }
        self.send(Message::Binary(data.into())).await?;
        if !self.close_if_complete() {
            self.read_next();
        }
        Ok(())
    }

    /// Starts the download's next read, of at most what is left.
    fn read_next(&mut self) {
        let Some(Active {
            transfer,
            stage: Stage::Open(handle),
        }) = &self.active
        else {
            return;
        };
        let (asked, offset) = (transfer.next_read(), transfer.moved);
        let (remote, handle) = (Arc::clone(&self.remote), handle.clone());
        self.start(async move { Done::Read(remote.read(&handle, offset, asked).await, asked) });
    }

    /// Closes the open file once every byte has moved; `true` when it started the close.
    fn close_if_complete(&mut self) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        let Stage::Open(handle) = &active.stage else {
            return false;
        };
        if !active.transfer.is_complete() {
            return false;
        }
        let (remote, handle) = (Arc::clone(&self.remote), handle.clone());
        active.stage = Stage::Closing(Outcome::Done);
        self.start(async move { Done::Closed(remote.close(&handle).await) });
        true
    }

    /// The file closed: the audit entry, then `done`. A finished upload the device did not
    /// close cleanly has failed; a download's bytes are all sent either way, and a cancelled
    /// transfer is over whether or not the device took the close or the removal.
    async fn closed(&mut self, closed: Result<(), Broken>) -> Result<(), Cause> {
        let Some(Active {
            transfer,
            stage: Stage::Closing(outcome),
        }) = self.active.take()
        else {
            return Err(Cause::GatewayError);
        };
        if let Err(broken) = closed
            && transfer.direction == Direction::Upload
            && outcome == Outcome::Done
        {
            self.active = Some(Active {
                transfer,
                stage: Stage::Closing(outcome),
            });
            return Err(self.broken(broken));
        }
        if !self.audit(&transfer, outcome, None).await {
            return Err(Cause::GatewayError);
        }
        self.send_text(protocol::DONE).await
    }
}
