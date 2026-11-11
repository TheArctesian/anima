//! `deleteRecording` (K1, D-13, D-15, G-8): the row, which holds the only sealed key, goes
//! first with its audit entry `recording.delete`; only after that commits are its pending
//! playback tickets withdrawn and its file removed. A file left behind by a failed removal
//! cannot be read, and the retention job sweeps a `.anrec` file that has no row.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde_json::json;
use sqlx::PgPool;

use super::{
    Error,
    model::{Catalogued, file_name},
    store,
    tickets::Tickets,
};
use crate::{
    events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
    ids::RecordingId,
};

/// The event of a deleted recording: no path, size or content (LOG-007).
const DELETED: &str = "A session recording was deleted. Its key is gone, so its file cannot be \
                       read again.";

/// Deletes the recording `id`: its row and the audit entry of `trail`'s operator, client and
/// time in one transaction; then every pending playback ticket for it in `playback`, and its
/// file in `root`. A file that is already gone is fine; a file that cannot be removed is
/// logged by its kind of failure only, and the delete still succeeds, since the key is gone.
///
/// A recording still being written can be deleted. Its writer keeps its open file and goes
/// on writing to it, unlinked, until the session ends: nothing can read those parts back, as
/// the key went with the row, and the space is freed when the writer closes the file. The
/// writer's end of the row then updates no row. A ticket issued while the delete commits may
/// outlive this withdrawal; playback reads the row again when it redeems one (G-7).
///
/// # Errors
/// [`Error::NotFound`] for an unknown id, and then nothing changes; `Events`, `Database` or
/// `StoredValue` when the transaction fails, and then the row and the file stay.
pub(super) async fn delete_recording(
    db: &PgPool,
    (root, playback): (Option<&Path>, &Tickets),
    trail: Trail<'_>,
    id: RecordingId,
) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    let deleted = store::delete(&mut *tx, id)
        .await?
        .ok_or(Error::NotFound(id))?;
    trail
        .write(&mut tx, entry(&deleted))
        .await
        .map_err(Error::Events)?;
    tx.commit().await?;
    playback.withdraw_recording(id);
    if let Some(root) = root {
        remove_file(root.join(file_name(id)), id).await;
    }
    Ok(())
}

/// The audit entry `recording.delete` and its Info event.
fn entry(deleted: &Catalogued) -> TrailEntry {
    TrailEntry {
        source: Source::Session,
        severity: Severity::Info,
        device: deleted
            .device
            .map(|device| (device, deleted.device_name.clone())),
        message: DELETED.to_owned(),
        action: AuditAction::Delete,
        target: AuditTarget::Recording(deleted.id),
        change: json!({
            "sessionId": deleted.session.to_string(),
            "deviceName": deleted.device_name,
            "protocol": deleted.protocol.as_str(),
            "startedAt": deleted.started_at,
        }),
    }
}

/// Removes `path` off the async threads. Never logs the path (LOG-007, K7).
async fn remove_file(path: PathBuf, id: RecordingId) {
    let removed = tokio::task::spawn_blocking(move || std::fs::remove_file(path)).await;
    // A panic in the blocking task reads as `other`: the file may still be there.
    let removed = removed.unwrap_or_else(|_| Err(io::ErrorKind::Other.into()));
    match removed {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            let kind = error.kind();
            tracing::warn!(recording = %id, %kind, "a deleted recording's file could not be removed; the retention job removes it");
        }
    }
}
