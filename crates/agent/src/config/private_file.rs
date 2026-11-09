//! Files only the running user can read: created `0600` from the start in the folder they will
//! live in, then renamed into place (MET-013, K8 Section 2).

use std::fs::File;
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::Path;

use zeroize::Zeroizing;

/// The only mode a private file has.
const MODE: u32 = 0o600;

/// Whether a private file with this `mode` and `owner` is not safe to trust when the agent runs
/// as `running`: the mode must be exactly 0600 and the owner the running user.
#[must_use]
pub const fn exposed(mode: u32, owner: u32, running: u32) -> bool {
    mode & 0o777 != MODE || owner != running
}

/// Whether a private folder with this `mode` and `owner` is not safe to trust: no access for
/// group or others, and the running user as owner.
#[must_use]
pub const fn folder_exposed(mode: u32, owner: u32, running: u32) -> bool {
    mode & 0o077 != 0 || owner != running
}

/// Whether a folder with this `mode` lets any user replace what is in it: others can write
/// and the sticky bit (which keeps users to their own entries, as in `/tmp`) is not set.
#[must_use]
pub const fn parent_replaceable(mode: u32) -> bool {
    mode & 0o002 != 0 && mode & 0o1000 == 0
}

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
    if exposed(
        metadata.permissions().mode(),
        metadata.uid(),
        running_user()?,
    ) {
        return Err(PrivateFileError::Exposed);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

/// The user ID of this process: the owner of a file it creates (an unnamed temporary file, so
/// this works on Linux and macOS alike).
///
/// # Errors
/// An I/O error when the temporary file cannot be made.
pub fn running_user() -> io::Result<u32> {
    Ok(tempfile::tempfile()?.metadata()?.uid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_013_a_file_is_exposed_unless_it_is_0600_and_the_runner_s() {
        assert!(!exposed(0o100_600, 1000, 1000));
        assert!(exposed(0o100_600, 1001, 1000), "another owner");
        assert!(exposed(0o100_644, 1000, 1000), "readable by others");
        assert!(exposed(0o100_400, 1000, 1000), "not exactly 0600");
        assert!(exposed(0o100_660, 1000, 1000));
    }

    #[test]
    fn met_013_a_folder_is_exposed_when_group_or_others_have_any_access_or_it_is_not_the_runner_s()
    {
        assert!(!folder_exposed(0o040_700, 1000, 1000));
        assert!(!folder_exposed(0o040_500, 1000, 1000));
        assert!(folder_exposed(0o040_750, 1000, 1000));
        assert!(folder_exposed(0o040_701, 1000, 1000));
        assert!(folder_exposed(0o040_700, 0, 1000));
    }

    #[test]
    fn met_013_a_parent_others_can_write_is_replaceable_unless_it_is_sticky() {
        assert!(parent_replaceable(0o040_777));
        assert!(parent_replaceable(0o040_702));
        assert!(!parent_replaceable(0o041_777), "sticky, like /tmp");
        assert!(
            !parent_replaceable(0o040_775),
            "group may write, others may not"
        );
        assert!(!parent_replaceable(0o040_755));
    }
}
