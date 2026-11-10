//! Opening a recording (G-4, K4 §11): its key, its row, then its file.

use std::{
    fs::{File, OpenOptions},
    io::{self, Write as _},
    os::unix::fs::OpenOptionsExt as _,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::Origin;
use crate::{
    credentials::{Sealed, Secret, Vault},
    ids::RecordingId,
    retention::{self, Subject},
    sessions::{
        Serving,
        recording::{
            Error,
            format::{Header, NONCE_PREFIX_LEN, RecordingKey, Sealer},
            model, store,
        },
    },
};

/// Bytes in a recording key (K4 §11).
const KEY_LEN: usize = 32;

/// The recording's file and the sealer that writes it. The sealer owns the key.
pub(super) struct Sink {
    pub(super) file: Arc<File>,
    pub(super) sealer: Sealer,
}

/// A recording whose row is written. `file` is `None` when the file could not be created.
pub(super) struct Opened {
    pub(super) id: RecordingId,
    pub(super) file: Option<Sink>,
}

/// Makes the key and nonce prefix, seals the key, inserts the row with its expiry, then
/// creates the file and writes its header.
pub(super) async fn open(serving: &Serving, origin: &Origin) -> Result<Opened, Error> {
    let root = serving.recording.root.clone().ok_or(Error::Unavailable)?;
    let id = RecordingId::from(Uuid::now_v7());
    let started_at = serving.clock.now();
    let (key, key_sealed) = new_key(&serving.vault, id)?;
    let mut nonce_prefix = [0_u8; NONCE_PREFIX_LEN];
    getrandom::fill(&mut nonce_prefix).map_err(Error::Random)?;
    let format = model::format_of(origin.protocol);
    let new = store::NewRecording {
        id,
        session: origin.session,
        device: origin.device,
        device_name: &origin.device_name,
        protocol: origin.protocol,
        format,
        key_sealed: &key_sealed,
        started_at,
        expires_at: expiry(serving, started_at).await?,
    };
    store::insert(&serving.db, &new).await?;
    let sealer = Sealer::new(key, &Header::new(format, id, nonce_prefix));
    let file = sink(root, id, sealer).await;
    Ok(Opened { id, file })
}

/// Creates the file off the async threads; `None`, logged, when it cannot be created.
async fn sink(root: PathBuf, id: RecordingId, sealer: Sealer) -> Option<Sink> {
    let header = *sealer.header();
    let created = tokio::task::spawn_blocking(move || create_file(&root, id, &header)).await;
    // A panic in the blocking task reads as `other`: it left no file the hub can use.
    let created = created.unwrap_or_else(|_| Err(io::ErrorKind::Other.into()));
    match created {
        Ok(file) => Some(Sink {
            file: Arc::new(file),
            sealer,
        }),
        Err(error) => {
            tracing::warn!(recording = %id, kind = %error.kind(), "a recording file could not be created");
            None
        }
    }
}

/// 32 bytes from the OS random source, and the same key sealed for this recording's row. The
/// source bytes are wiped; the key and the secret wipe their own copies.
fn new_key(vault: &Vault, id: RecordingId) -> Result<(RecordingKey, Sealed), Error> {
    let mut source = Zeroizing::new([0_u8; KEY_LEN]);
    getrandom::fill(&mut *source).map_err(Error::Random)?;
    let secret = Secret::new(source.to_vec());
    let sealed = vault
        .seal(&secret, key_context(id).as_bytes())
        .map_err(Error::Vault)?;
    Ok((RecordingKey::from_bytes(*source), sealed))
}

/// The vault context of a recording's key (K3).
pub(in crate::sessions::recording) fn key_context(id: RecordingId) -> String {
    format!("sessions.recordings:{id}")
}

/// `started_at` plus the recordings period (G-6, CR-19).
async fn expiry(serving: &Serving, started_at: DateTime<Utc>) -> Result<DateTime<Utc>, Error> {
    let periods = retention::periods(&serving.db)
        .await
        .map_err(Error::Periods)?;
    started_at
        .checked_add_days(periods.keep(Subject::Recordings))
        .ok_or(Error::Expiry)
}

/// Creates `<root>/<id>.anrec` with mode 0600 and writes `header`. Never opens a file that
/// exists: a key over an existing file would reuse its nonces (K4 §11).
pub(in crate::sessions::recording) fn create_file(
    root: &Path,
    id: RecordingId,
    header: &[u8],
) -> io::Result<File> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join(format!("{id}.anrec")))?;
    file.write_all(header)?;
    file.sync_data()?;
    Ok(file)
}
