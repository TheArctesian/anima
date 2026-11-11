//! The writer task (K4 §11): seals and writes each part, checks the free space before each
//! part (the last too, when it holds bytes), writes the last part, then ends the row. On a
//! stop it also writes a Warning event and publishes `sessionChanged` (G-5). Logs only here,
//! with the recording and session IDs at most (LOG-007).

use std::{io::Write as _, sync::Arc};

use super::{
    Origin,
    opening::Opened,
    queue::{Job, Plain},
    spool::Spool,
};
use crate::{
    events::{self, NewEvent, Severity, Source},
    sessions::{
        Serving,
        recording::{
            Error,
            model::StopReason,
            space::{FreeSpace, MIN_FREE},
            store::{self, Ending},
        },
        store as sessions_store,
    },
};

pub(super) struct Writer {
    pub(super) serving: Serving,
    pub(super) origin: Origin,
    pub(super) spool: Arc<Spool>,
    pub(super) opened: Opened,
    pub(super) free: FreeSpace,
}

impl Writer {
    pub(super) async fn run(mut self) {
        let mut written = 0_u64;
        loop {
            match self.spool.next().await {
                Job::Part(part) => match self.write_part(&part).await {
                    Ok(()) => written += part.len() as u64,
                    Err(reason) => self.stop(reason),
                },
                Job::Last { tail, stopped } => {
                    let ending = self.write_end(tail, stopped, written).await;
                    self.end(&ending).await;
                    return;
                }
            }
        }
    }

    /// Writes the last part, with `tail` when the disk has room for it as for any part, and
    /// says how the recording ended after `written` bytes.
    async fn write_end(
        &mut self,
        mut tail: Plain,
        mut stopped: Option<StopReason>,
        written: u64,
    ) -> Ending {
        if !tail.is_empty()
            && self.opened.file.is_some()
            && let Err(reason) = self.check_space().await
        {
            self.stop(reason);
            // The first stop's reason stays, as in the queue.
            stopped = stopped.or(Some(reason));
            tail = Plain::default();
        }
        let complete = self.write_last(&tail).await;
        let tail_written = if complete { tail.len() as u64 } else { 0 };
        Ending {
            at: self.serving.clock.now(),
            size_bytes: written + tail_written,
            complete,
            stopped,
        }
    }

    /// A disk failure: nothing more is written but the last part, and a write error leaves
    /// even that out, since the file may end inside a part.
    fn stop(&mut self, reason: StopReason) {
        if reason == StopReason::WriteFailed {
            self.opened.file = None;
        }
        self.spool.fail(reason);
    }

    /// Checks the free space, then seals and writes one part and syncs it.
    async fn write_part(&mut self, part: &[u8]) -> Result<(), StopReason> {
        self.check_space().await?;
        let sink = self.opened.file.as_mut().ok_or(StopReason::WriteFailed)?;
        let framed = sink
            .sealer
            .seal(part)
            .map_err(|_| StopReason::WriteFailed)?;
        append(Arc::clone(&sink.file), framed).await
    }

    /// The free space before a part: under 1 GiB stops with `disk_full`, and a reading that
    /// fails stops with `write_failed`.
    async fn check_space(&self) -> Result<(), StopReason> {
        let root = self.serving.recording.root.clone();
        let free = self.free;
        let space = tokio::task::spawn_blocking(move || root.as_deref().map(free)).await;
        match space {
            Ok(Some(Ok(bytes))) if bytes >= MIN_FREE => Ok(()),
            Ok(Some(Ok(_))) => Err(StopReason::DiskFull),
            _ => Err(StopReason::WriteFailed),
        }
    }

    /// Seals and writes the last part, which wipes the key. Returns whether the file now has
    /// its last part.
    async fn write_last(&mut self, tail: &[u8]) -> bool {
        let Some(sink) = self.opened.file.take() else {
            return false;
        };
        let Ok(framed) = sink.sealer.finish(tail) else {
            return false;
        };
        append(sink.file, framed).await.is_ok()
    }

    /// Ends the row; on a stop also the Warning event, then `sessionChanged`.
    async fn end(&self, ending: &Ending) {
        let result = match ending.stopped {
            None => store::end(&self.serving.db, self.opened.id, ending)
                .await
                .map(drop),
            Some(reason) => {
                self.log_stop(reason);
                self.end_stopped(ending, reason).await
            }
        };
        if let Err(error) = result {
            let (id, session) = (self.opened.id, self.origin.session);
            tracing::warn!(recording = %id, session = %session, fault = error.fault(), "a recording's end could not be stored");
        }
    }

    fn log_stop(&self, reason: StopReason) {
        let (id, session) = (self.opened.id, self.origin.session);
        tracing::warn!(recording = %id, session = %session, reason = reason.as_str(), "a recording stopped");
    }

    async fn end_stopped(&self, ending: &Ending, reason: StopReason) -> Result<(), Error> {
        let serving = &self.serving;
        let event = NewEvent {
            severity: Severity::Warning,
            source: Source::Session,
            device: Some((self.origin.device, self.origin.device_name.clone())),
            message: reason.sentence().to_owned(),
        };
        let mut tx = serving.db.begin().await?;
        store::end(&mut *tx, self.opened.id, ending).await?;
        events::record(&mut tx, &serving.bus, ending.at, event)
            .await
            .map_err(Error::Events)?;
        tx.commit().await?;
        let row = sessions_store::get(&serving.db, self.origin.session)
            .await
            .map_err(|error| Error::Session(Box::new(error)))?;
        if let Some(row) = row {
            serving.live.publish_change(row);
        }
        Ok(())
    }
}

/// Appends `framed` to the file and syncs it, off the async threads.
async fn append(file: Arc<std::fs::File>, framed: Vec<u8>) -> Result<(), StopReason> {
    let written = tokio::task::spawn_blocking(move || {
        (&*file).write_all(&framed)?;
        file.sync_data()
    })
    .await;
    match written {
        Ok(Ok(())) => Ok(()),
        _ => Err(StopReason::WriteFailed),
    }
}
