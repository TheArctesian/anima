//! Files that only the running user may read: the folder is `0700`, the file `0600` from the
//! moment it exists, and it is replaced by rename so a reader never sees half of it (CMD-012,
//! as the agent does for MET-013).

use std::fs::{self, DirBuilder, File};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::Path;

use zeroize::Zeroizing;

/// The most a file of the command line may hold; a config or token file is a few kilobytes.
pub const LIMIT: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PrivateFileError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("it is not mode 0600 and owned by you; fix it with chmod 600")]
    Exposed,
    #[error("it is larger than {LIMIT} bytes")]
    TooLarge,
}

/// Reads `path`, or `None` when it does not exist. A file others can read, or that another user
/// owns, is refused: a token in it could already be taken or replaced.
///
/// # Errors
/// [`PrivateFileError`] when it is exposed, too large or cannot be read.
pub fn read(path: &Path) -> Result<Option<Zeroizing<Vec<u8>>>, PrivateFileError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if metadata.permissions().mode() & 0o077 != 0 || metadata.uid() != running_user()? {
        return Err(PrivateFileError::Exposed);
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(LIMIT.min(64 * 1024)));
    file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(PrivateFileError::TooLarge);
    }
    Ok(Some(bytes))
}

/// Writes `bytes` as `dir/name`: the folder is made `0700` when missing; the bytes go into a
/// `0600` temporary file in it, which is flushed and renamed over `name`.
///
/// # Errors
/// [`PrivateFileError::Io`] when a step fails; the old file is then left as it was.
pub fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), PrivateFileError> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    // `tempfile` creates the file `0600` and exclusively, so no other process can have it.
    let mut temporary = tempfile::Builder::new().prefix(".tmp-").tempfile_in(dir)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(dir.join(name))
        .map_err(|error| error.error)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// Deletes `path`; a missing file is already deleted.
///
/// # Errors
/// [`PrivateFileError::Io`] when it cannot be deleted.
pub fn remove(path: &Path) -> Result<(), PrivateFileError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// The user ID of this process: the owner of a file it creates.
fn running_user() -> io::Result<u32> {
    Ok(tempfile::tempfile()?.metadata()?.uid())
}
