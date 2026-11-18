//! Where the sealed pieces are kept (FIL-001, R-145): one interface, so that the place can
//! change later, and its one form now, a folder of the hub (`ANIMA_FILES_DIR`).

use std::{
    io,
    os::unix::fs::DirBuilderExt as _,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{StreamExt as _, stream};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt as _, AsyncWriteExt as _},
};

use super::parts::SealedStream;
use crate::engine::sealing_keys::is_name;

/// Bytes read from a stored piece at a time.
const READ_SIZE: usize = 64 * 1024;

/// Why the store cannot do what was asked. Holds no byte of a piece and no path.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("no piece has this key and hash")]
    NotFound,
    #[error("a key name or a hash is not in its form")]
    BadName,
    #[error("the store cannot be read or written ({0})")]
    Io(io::ErrorKind),
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::Io(error.kind())
        }
    }
}

/// The store of sealed pieces. `hash` is the SHA-256 of the plain bytes in lower-case
/// hexadecimal; a piece is one file of one key.
#[async_trait]
pub trait Pieces: Send + Sync {
    /// Stores the piece; a piece that exists is replaced by the same bytes. Returns the sealed
    /// size.
    async fn put(
        &self,
        key_name: &str,
        hash: &str,
        sealed: SealedStream,
    ) -> Result<u64, StoreError>;
    async fn get(&self, key_name: &str, hash: &str) -> Result<SealedStream, StoreError>;
    /// Removes the piece; a piece that does not exist is not an error.
    async fn remove(&self, key_name: &str, hash: &str) -> Result<(), StoreError>;
}

/// Whether `hash` is a SHA-256 in lower-case hexadecimal, so that it is never a path.
#[must_use]
pub fn is_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The pieces in a folder: `<root>/<key_name>/<first two characters>/<hash>`. A piece is
/// written under a temporary name, then renamed, so that no reader sees half of one.
#[derive(Clone, Debug)]
pub struct FolderPieces {
    root: PathBuf,
}

impl FolderPieces {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, key_name: &str, hash: &str) -> Result<PathBuf, StoreError> {
        if !is_name(key_name) || !is_hash(hash) {
            return Err(StoreError::BadName);
        }
        Ok(self.root.join(key_name).join(&hash[..2]).join(hash))
    }
}

/// Makes `folder` and its parents for the hub user only.
pub(super) async fn private_folder(folder: &Path) -> io::Result<()> {
    let folder = folder.to_path_buf();
    tokio::task::spawn_blocking(move || {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(folder)
    })
    .await
    .map_err(io::Error::other)?
}

/// A name in `folder` that no other write uses.
pub(super) fn temporary_name(folder: &Path, stem: &str) -> io::Result<PathBuf> {
    let mut marker = [0_u8; 12];
    getrandom::fill(&mut marker).map_err(io::Error::other)?;
    Ok(folder.join(format!(".{stem}.{}.tmp", super::hex(&marker))))
}

/// The bytes of `file` as a stream, in parts of [`READ_SIZE`].
pub(super) fn file_stream(file: File) -> SealedStream {
    Box::pin(stream::try_unfold(file, |mut file| async move {
        let mut buffer = vec![0_u8; READ_SIZE];
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            return Ok(None);
        }
        buffer.truncate(read);
        Ok(Some((Bytes::from(buffer), file)))
    }))
}

/// Writes `sealed` to the new file `path` and gives its size; the file is removed when a
/// write fails.
pub(super) async fn write_new(path: &Path, mut sealed: SealedStream) -> io::Result<u64> {
    let written = async {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .await?;
        let mut size = 0_u64;
        while let Some(chunk) = sealed.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            size += chunk.len() as u64;
        }
        file.sync_all().await?;
        Ok(size)
    }
    .await;
    if written.is_err() {
        // The first error is the one to give; a file that is not there is not a second one.
        let _ = fs::remove_file(path).await;
    }
    written
}

#[async_trait]
impl Pieces for FolderPieces {
    async fn put(
        &self,
        key_name: &str,
        hash: &str,
        sealed: SealedStream,
    ) -> Result<u64, StoreError> {
        let path = self.path(key_name, hash)?;
        let folder = path.parent().ok_or(StoreError::BadName)?;
        private_folder(folder).await?;
        let temporary = temporary_name(folder, hash)?;
        let size = write_new(&temporary, sealed).await?;
        if let Err(error) = fs::rename(&temporary, &path).await {
            let _ = fs::remove_file(&temporary).await;
            return Err(error.into());
        }
        Ok(size)
    }

    async fn get(&self, key_name: &str, hash: &str) -> Result<SealedStream, StoreError> {
        let file = File::open(self.path(key_name, hash)?).await?;
        Ok(file_stream(file))
    }

    async fn remove(&self, key_name: &str, hash: &str) -> Result<(), StoreError> {
        match fs::remove_file(self.path(key_name, hash)?).await {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }
}
