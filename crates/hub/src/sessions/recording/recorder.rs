//! A session's recorder (K4 §11, G-4): the handle the relay feeds and the task that writes.
//!
//! [`start`] makes the key, inserts the row, then creates the file. The relay hands bytes to
//! [`Recorder::push`], which never waits on the disk; the writer task seals and writes each
//! part. The 5-second flush lives in the writer task, so the relay needs no timer. A stop
//! (`size_limit`, `disk_full`, `write_failed`) ends the recording, not the session; the session
//! end calls [`Recorder::finish`] before it writes the session row (K4 §5).

mod opening;
mod queue;
mod spool;
mod writer;

use std::sync::Arc;

use tokio::task::JoinHandle;

use super::{Error, space::FreeSpace};
use crate::{
    ids::{DeviceId, RecordingId, SessionId},
    sessions::{Protocol, Serving},
};

use spool::Spool;

#[cfg(test)]
pub(super) use opening::{create_file, key_context};
#[cfg(test)]
pub(super) use queue::{FLUSH_AFTER, Job, Next, Pushed, QUEUE_PARTS, Queue};

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
/// [`Error`] when no row was written: recording is unavailable, the random source, the vault,
/// the retention periods or the database failed.
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
pub(super) async fn begin(
    serving: &Serving,
    origin: Origin,
    free: FreeSpace,
) -> Result<Recorder, Error> {
    let opened = opening::open(serving, &origin).await?;
    let id = opened.id;
    let spool = Arc::new(Spool::new(serving.recording.max_bytes));
    if opened.file.is_none() {
        spool.fail(super::model::StopReason::WriteFailed);
    }
    let writer = writer::Writer {
        serving: serving.clone(),
        origin,
        spool: Arc::clone(&spool),
        opened,
        free,
    };
    Ok(Recorder {
        id,
        spool,
        writer: Some(tokio::spawn(writer.run())),
    })
}

impl Recorder {
    #[cfg(test)]
    pub(super) const fn id(&self) -> RecordingId {
        self.id
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
