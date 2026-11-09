//! Files only the running user can read: created `0600` from the start in the folder they will
//! live in, then renamed into place (MET-013, K8 Section 2).

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::Path;

use zeroize::Zeroizing;

/// The only mode a private file has.
const MODE: u32 = 0o600;

#[derive(Debug, thiserror::Error)]
pub enum PrivateFileError {
    #[error("file access failed: {0}")]
    Io(#[from] io::Error),
    #[error("the file is not mode 0600 and owned by the user running the agent")]
    Exposed,
}

/// Writes `bytes` as `dir/name`: into a temporary file in `dir` that is created `0600` (so the
/// bytes never sit in a file others can read), flushed to disk, then renamed over `name`.
///
/// # Errors
/// [`PrivateFileError::Io`] when any step fails; the old file is then left as it was.
pub fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), PrivateFileError> {
    // `tempfile` creates the file with mode 0600, exclusively, so no other process can have it.
    let mut temporary = tempfile::Builder::new().prefix(".tmp-").tempfile_in(dir)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(dir.join(name))
        .map_err(|error| error.error)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// Reads `path`; `None` when it does not exist. Refuses a file that is not mode `0600` or not
/// owned by the user running the agent, since a token another user could read is already lost.
///
/// # Errors
/// [`PrivateFileError::Exposed`] for such a file, [`PrivateFileError::Io`] for a read failure.
pub fn read(path: &Path) -> Result<Option<Zeroizing<Vec<u8>>>, PrivateFileError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if metadata.permissions().mode() & 0o777 != MODE || metadata.uid() != running_user()? {
        return Err(PrivateFileError::Exposed);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

/// The user ID of this process: the owner of its `/proc/self` folder (Linux).
fn running_user() -> io::Result<u32> {
    Ok(fs::metadata("/proc/self")?.uid())
}
