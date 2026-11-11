//! A session's recorder (K4 §11, G-4): the handle the relay feeds and the task that writes.
//!
//! [`start`] makes the key, inserts the row, then creates the file, in the task that then
//! writes the recording, so a written row always ends. The relay hands bytes to
//! [`Recorder::push`], which never waits on the disk; the writer task seals and writes each
//! part. The 5-second flush lives in the writer task, so the relay needs no timer. A stop
//! (`size_limit`, `disk_full`, `write_failed`) ends the recording, not the session; the session
//! end calls [`Recorder::finish`] before it writes the session row (K4 §5).

mod opening;
mod queue;
mod spool;
mod writer;

use std::sync::Arc;

use tokio::{sync::oneshot, task::JoinHandle};
use uuid::Uuid;

use super::{Error, space::FreeSpace};
use crate::{
    ids::{DeviceId, RecordingId, SessionId},
    sessions::{Protocol, Serving},
};

use spool::Spool;

#[cfg(test)]
pub(super) use opening::{create_file, key_context};
#[cfg(test)]
pub(super) use queue::{FLUSH_AFTER, Job, Next, Pushed, Queue};

/// The session a recording is of, as the tunnel knows it when its relay starts.
#[derive(Clone, Debug)]
pub(in crate::sessions) struct Origin {
    pub(in crate::sessions) session: SessionId,
    pub(in crate::sessions) device: DeviceId,
    pub(in crate::sessions) device_name: String,
    pub(in crate::sessions) protocol: Protocol,
}

/// The relay's side of a recording. No `Debug`: it holds recording content.
///
/// Dropping it without [`Recorder::finish`] closes the recording too; the writer then ends it
/// on its own.
pub(in crate::sessions) struct Recorder {
    id: RecordingId,
    spool: Arc<Spool>,
    writer: Option<JoinHandle<()>>,
}

/// Starts recording `origin` (G-4): a fresh key sealed with the vault, the row with its
/// expiry, then the file `<root>/<id>.anrec` with its header. A file that cannot be created
/// leaves the row stopped with `write_failed` and returns a recorder that keeps nothing.
///
/// # Errors
/// [`Error`] when recording is unavailable, the random source, the vault, the retention
/// periods or the database failed, or when the runtime shut down while starting. A row
/// written before that failure is ended, not left open; [`Error::ShuttingDown`] may follow a
/// row written before the start was cancelled.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the tunnels' tee (WP-48 T7, T8) is the first caller"
    )
)]
pub(in crate::sessions) async fn start(
    serving: &Serving,
    origin: Origin,
) -> Result<Recorder, Error> {
    begin(serving, origin, super::space::free_space).await
}

/// [`start`] with the free space read by `free`.
///
/// Cancel-safe: one task opens the recording, then writes it, so a row once written always
/// has a writer to end it. The recorder exists before the first `await`; if the caller drops
/// this future, the recorder's drop closes the spool and the writer ends the row at once.
pub(super) async fn begin(
    serving: &Serving,
    origin: Origin,
    free: FreeSpace,
) -> Result<Recorder, Error> {
    let id = RecordingId::from(Uuid::now_v7());
    let spool = Arc::new(Spool::new(serving.recording.max_bytes));
    let (told, opened) = oneshot::channel();
    let opening = Opening {
        serving: serving.clone(),
        origin,
        spool: Arc::clone(&spool),
        free,
        id,
    };
    let mut recorder = Recorder {
        id,
        spool,
        writer: Some(tokio::spawn(opening.run(told))),
    };
    match opened.await {
        Ok(Ok(())) => Ok(recorder),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(recorder.unopened().await),
    }
}

/// A recording before its row: what its task needs to open it, then write it.
struct Opening {
    serving: Serving,
    origin: Origin,
    spool: Arc<Spool>,
    free: FreeSpace,
    id: RecordingId,
}

impl Opening {
    /// Opens the recording, tells [`begin`] how that went, then writes it until it ends.
    async fn run(self, told: oneshot::Sender<Result<(), Error>>) {
        let opened = match opening::open(&self.serving, &self.origin, self.id).await {
            Ok(opened) => opened,
            Err(error) => {
                // No row was written. Only when `begin` was dropped is there nobody to tell.
                if let Err(Err(error)) = told.send(Err(error)) {
                    tracing::warn!(recording = %self.id, fault = error.fault(), "a recording could not start after its caller left");
                }
                return;
            }
        };
        if opened.file.is_none() {
            self.spool.fail(super::model::StopReason::WriteFailed);
        }
        // A send to a dropped `begin` fails; its recorder closed the spool, so the writer
        // below ends the row at once.
        drop(told.send(Ok(())));
        let writer = writer::Writer {
            serving: self.serving,
            origin: self.origin,
            spool: self.spool,
            opened,
            free: self.free,
        };
        writer.run().await;
    }
}

impl Recorder {
    #[cfg(test)]
    pub(super) const fn id(&self) -> RecordingId {
        self.id
    }

    /// Why the opening task ended without a word to [`begin`]: a panic goes on as a panic in
    /// [`begin`], as it would have inline; otherwise the runtime is shutting down.
    async fn unopened(&mut self) -> Error {
        if let Some(task) = self.writer.take()
            && let Err(ended) = task.await
            && ended.is_panic()
        {
            std::panic::resume_unwind(ended.into_panic());
        }
        Error::ShuttingDown
    }

    /// Hands `bytes` to the recording: a Guacamole instruction as relayed, or an asciicast
    /// line. Never waits. After a stop, bytes are dropped silently.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the tunnels' tee (WP-48 T7, T8) is the first caller"
        )
    )]
    pub(in crate::sessions) fn push(&self, bytes: &[u8]) {
        self.spool.push(bytes);
    }

    /// Writes the last part and ends the row; returns when the recording is ended. The key is
    /// wiped with the last part.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the session end (WP-48 T7, T8) is the first caller"
        )
    )]
    pub(in crate::sessions) async fn finish(mut self) {
        self.spool.close();
        if let Some(writer) = self.writer.take()
            && writer.await.is_err()
        {
            tracing::error!(recording = %self.id, "a recording's writer ended without its last part");
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.spool.close();
    }
}
