//! The device's file system over SFTP (K4 Section 9): the requests the files tunnel makes, one
//! at a time, and what each answer means for the client. I/O edge; nothing here logs.
//!
//! The device's own words (SFTP status messages) are dropped: an answer becomes a K4 refusal
//! or [`Broken`], never text (LOG-007).

use std::time::Duration;

use chrono::DateTime;
use russh::{ChannelStream, client::Msg};
use russh_sftp::{
    client::{Config, RawSftpSession, error::Error as SftpError},
    protocol::{File, FileAttributes, OpenFlags, StatusCode},
};

use super::protocol::{DevicePath, Entry, Kind, Listing, MAX_PATH_BYTES, Refusal};

/// Largest SFTP packet the hub reads from the device; past it the session fails.
const MAX_PACKET: u32 = 256 * 1024;
/// Longest entry name the hub lists; longer ones are left out, which bounds a listing's size.
const MAX_NAME_BYTES: usize = 1024;
/// The file type bits of a POSIX mode, and the three types the hub lists.
const TYPE_BITS: u32 = 0o170_000;
const REGULAR: u32 = 0o100_000;
const DIRECTORY: u32 = 0o040_000;
const SYMLINK: u32 = 0o120_000;

/// The files tunnel cannot go on with the device: the SFTP session or the SSH link failed, the
/// device broke the SFTP protocol or did not answer in time, or the hub could not read the
/// device's settings. Holds nothing from the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum Broken {
    #[error("the SFTP session with the device failed")]
    Sftp,
    #[error("the device's settings could not be read")]
    Devices,
}

/// An answer the client gets: the value, or a K4 refusal; or the tunnel cannot go on.
pub(super) type Asked<T> = Result<Result<T, Refusal>, Broken>;

/// An open file on the device. Shows nothing of the device's handle bytes.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Handle(String);

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handle")
    }
}

/// The SFTP session on the device, over the link's `sftp` channel.
pub(super) struct Remote(RawSftpSession);

/// Starts SFTP on `stream`: each request waits at most `wait` for its answer.
///
/// # Errors
/// [`Broken::Sftp`] when the device does not answer the version exchange.
pub(super) async fn start(stream: ChannelStream<Msg>, wait: Duration) -> Result<Remote, Broken> {
    let config = Config {
        max_packet_len: MAX_PACKET,
        request_timeout_secs: wait.as_secs().max(1),
        ..Config::default()
    };
    let sftp = RawSftpSession::new_with_config(stream, config);
    sftp.init().await.map_err(|_| Broken::Sftp)?;
    Ok(Remote(sftp))
}

/// What a failed request means: a status the device sent is a refusal (no such file, or any
/// other status as `DENIED`); anything else is the session failing.
fn refusal(error: &SftpError) -> Result<Refusal, Broken> {
    match error {
        SftpError::Status(status) => match status.status_code {
            StatusCode::NoSuchFile => Ok(Refusal::NotFound),
            StatusCode::NoConnection | StatusCode::ConnectionLost => Err(Broken::Sftp),
            _ => Ok(Refusal::Denied),
        },
        _ => Err(Broken::Sftp),
    }
}

/// The answer to a request whose failure the client is told.
fn asked<T>(answer: Result<T, SftpError>) -> Asked<T> {
    match answer {
        Ok(value) => Ok(Ok(value)),
        Err(error) => refusal(&error).map(Err),
    }
}

/// The kind of an entry with these attributes, when the hub lists it.
pub(super) fn kind(attrs: &FileAttributes) -> Option<Kind> {
    match attrs.permissions? & TYPE_BITS {
        REGULAR => Some(Kind::File),
        DIRECTORY => Some(Kind::Dir),
        SYMLINK => Some(Kind::Link),
        _ => None,
    }
}

/// The entry for `file`, unless the hub leaves it out: `.` and `..`, a name that is empty,
/// longer than [`MAX_NAME_BYTES`] or holds `/` or NUL, or a kind it does not list.
pub(super) fn entry(file: File) -> Option<Entry> {
    let name = file.filename;
    let listed = !matches!(name.as_str(), "" | "." | "..")
        && name.len() <= MAX_NAME_BYTES
        && !name.contains(['/', '\0']);
    let kind = kind(&file.attrs).filter(|_| listed)?;
    let modified_at = file
        .attrs
        .mtime
        .and_then(|seconds| DateTime::from_timestamp(i64::from(seconds), 0));
    Some(Entry {
        name,
        kind,
        size: file.attrs.size,
        modified_at,
    })
}

impl Remote {
    /// The directory at `path` as the device resolves it, with at most `cap` entries in the
    /// device's order; the rest are not read.
    pub(super) async fn list(&self, path: &DevicePath, cap: usize) -> Asked<Listing> {
        let resolved = match asked(self.0.realpath(path.as_str()).await)? {
            Ok(name) => name.files.into_iter().next().map(|file| file.filename),
            Err(refused) => return Ok(Err(refused)),
        };
        let Some(resolved) = resolved.filter(|path| path.len() <= MAX_PATH_BYTES) else {
            return Ok(Err(Refusal::Denied));
        };
        let handle = match asked(self.0.opendir(resolved.as_str()).await)? {
            Ok(handle) => handle.handle,
            Err(refused) => return Ok(Err(refused)),
        };
        let read = self.entries(&handle, cap).await;
        // The listing stands whether or not the device takes the close.
        let _closed = self.0.close(handle).await;
        Ok(read?.map(|entries| Listing {
            path: resolved,
            entries,
        }))
    }

    /// Reads the open directory until its end or `cap` entries.
    async fn entries(&self, handle: &str, cap: usize) -> Asked<Vec<Entry>> {
        let mut entries = Vec::new();
        while entries.len() < cap {
            match self.0.readdir(handle).await {
                Ok(name) => entries.extend(name.files.into_iter().filter_map(entry)),
                Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => break,
                Err(error) => return refusal(&error).map(Err),
            }
        }
        entries.truncate(cap);
        Ok(Ok(entries))
    }

    /// Opens the regular file at `path` (a link is followed) for reading, with its size.
    pub(super) async fn open_download(&self, path: &DevicePath) -> Asked<(Handle, u64)> {
        match asked(self.0.stat(path.as_str()).await)? {
            Ok(attrs) if kind(&attrs.attrs) == Some(Kind::File) => {}
            Ok(_) => return Ok(Err(Refusal::NotFound)),
            Err(refused) => return Ok(Err(refused)),
        }
        let flags = OpenFlags::READ;
        let handle = match asked(
            self.0
                .open(path.as_str(), flags, FileAttributes::default())
                .await,
        )? {
            Ok(handle) => Handle(handle.handle),
            Err(refused) => return Ok(Err(refused)),
        };
        let size = match asked(self.0.fstat(handle.0.as_str()).await) {
            Ok(Ok(attrs)) => attrs.attrs.size.ok_or(Refusal::Denied),
            Ok(Err(refused)) => Err(refused),
            Err(broken) => return Err(broken),
        };
        if size.is_err() {
            // Refused either way; the device may keep the handle until the session ends.
            let _closed = self.0.close(handle.0.as_str()).await;
        }
        Ok(size.map(|size| (handle, size)))
    }

    /// Creates the file at `path` for writing, only when nothing is there: exclusive
    /// creation, so the device itself refuses to replace a file (the hub never overwrites).
    pub(super) async fn create(&self, path: &DevicePath) -> Asked<Handle> {
        match self.0.lstat(path.as_str()).await {
            Ok(_) => return Ok(Err(Refusal::Exists)),
            Err(SftpError::Status(status)) if status.status_code == StatusCode::NoSuchFile => {}
            Err(error) => return refusal(&error).map(Err),
        }
        let flags = OpenFlags::CREATE | OpenFlags::EXCLUDE | OpenFlags::WRITE;
        let error = match self
            .0
            .open(path.as_str(), flags, FileAttributes::default())
            .await
        {
            Ok(handle) => return Ok(Ok(Handle(handle.handle))),
            Err(error) => refusal(&error)?,
        };
        // Something that appeared since the check made the exclusive open fail.
        Ok(Err(match self.0.lstat(path.as_str()).await {
            Ok(_) => Refusal::Exists,
            Err(_) => error,
        }))
    }

    /// Up to `len` bytes at `offset`; empty at the end of the file.
    ///
    /// # Errors
    /// [`Broken::Sftp`] for any failure: mid-transfer, the client has no refusal to read.
    pub(super) async fn read(
        &self,
        handle: &Handle,
        offset: u64,
        len: u32,
    ) -> Result<Vec<u8>, Broken> {
        match self.0.read(handle.0.as_str(), offset, len).await {
            Ok(data) => Ok(data.data),
            Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => {
                Ok(Vec::new())
            }
            Err(_) => Err(Broken::Sftp),
        }
    }

    /// Writes `data` at `offset`.
    ///
    /// # Errors
    /// [`Broken::Sftp`] for any failure, as [`Remote::read`].
    pub(super) async fn write(
        &self,
        handle: &Handle,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<(), Broken> {
        self.0
            .write(handle.0.as_str(), offset, data)
            .await
            .map(drop)
            .map_err(|_| Broken::Sftp)
    }

    /// Closes the file; for an upload, the device has then taken every byte.
    pub(super) async fn close(&self, handle: &Handle) -> Result<(), Broken> {
        self.0
            .close(handle.0.as_str())
            .await
            .map(drop)
            .map_err(|_| Broken::Sftp)
    }

    /// Removes the file at `path`: an upload the hub created and did not finish.
    pub(super) async fn remove(&self, path: &DevicePath) -> Result<(), Broken> {
        self.0
            .remove(path.as_str())
            .await
            .map(drop)
            .map_err(|_| Broken::Sftp)
    }

    /// A request with no effect, so the link sees traffic while the tunnel is idle.
    pub(super) async fn alive(&self) -> Result<(), Broken> {
        self.0
            .realpath(".")
            .await
            .map(drop)
            .map_err(|_| Broken::Sftp)
    }
}
