//! A copy of the hub's key (`vault.key`) at a place the operator chose in a native save
//! dialog (2.8, OR-S6-1). The key's bytes go from one file to the other; they never reach the
//! web view or a log (LOG-007).

use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use zeroize::Zeroizing;

use super::LocalHubError;

/// More than a key file holds; a larger file is not the hub's key.
const MOST_KEY: u64 = 4096;

/// Copies `key` to `destination` with mode 0600. The copy is written to a new file beside the
/// destination, then renamed onto it, so a reader never sees half a key. A destination that is
/// a symbolic link or not a plain file is refused, not followed.
pub fn copy(key: &Path, destination: &Path) -> Result<(), LocalHubError> {
    let bytes = read_key(key)?;
    match fs::symlink_metadata(destination) {
        Ok(found) if !found.file_type().is_file() => return Err(LocalHubError::Destination),
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(_) => return Err(LocalHubError::Destination),
    }
    let beside = temporary_beside(destination)?;
    let written = write_new(&beside, &bytes)
        .and_then(|()| fs::rename(&beside, destination).map_err(|_| LocalHubError::Destination));
    if written.is_err() {
        let _ = fs::remove_file(&beside);
    }
    written
}

fn read_key(key: &Path) -> Result<Zeroizing<Vec<u8>>, LocalHubError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(key)
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => LocalHubError::NoKey,
            _ => LocalHubError::Failed,
        })?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(1024));
    file.take(MOST_KEY + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LocalHubError::Failed)?;
    let fits = !bytes.is_empty() && bytes.len() as u64 <= MOST_KEY;
    if !fits {
        return Err(LocalHubError::Failed);
    }
    Ok(bytes)
}

/// A name in the destination's folder that no file has: `.<name>.<random>.part`.
fn temporary_beside(destination: &Path) -> Result<PathBuf, LocalHubError> {
    let name = destination.file_name().ok_or(LocalHubError::Destination)?;
    let mut random = [0_u8; 8];
    getrandom::fill(&mut random).map_err(|_| LocalHubError::Failed)?;
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(".{}.part", u64::from_le_bytes(random)));
    Ok(destination.with_file_name(temporary))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), LocalHubError> {
    let mut file: File = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| LocalHubError::Destination)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| LocalHubError::Destination)
}
