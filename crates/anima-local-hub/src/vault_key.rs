//! The hub's key file (`ANIMA_KEY_FILE`): 32 random bytes as base64. The hub seals the device
//! credentials and the TOTP secrets with it, so a new key cannot open an old database.

use std::io::{self, Write as _};
use std::path::Path;

use anima_database::owner_only;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

const KEY_BYTES: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// The database has content that only the lost key can open. No key is made.
    #[error(
        "the hub's key file is missing and the database is not new: put the key file back at {0}"
    )]
    Missing(String),
    #[error("the hub's key file cannot be made ({0})")]
    Write(io::ErrorKind),
    #[error("no random bytes for the hub's key")]
    Random,
}

/// Makes the key file at `path` when it is absent and the database is new.
///
/// `database_is_new` is the caller's knowledge that nothing was ever sealed: the cluster
/// folder was not there. The key is written before the database is made, so a crash between
/// the two leaves a key and no database, never a database and no key.
///
/// # Errors
/// [`KeyError::Missing`] when the key is absent and the database is not new.
pub fn prepare(path: &Path, database_is_new: bool) -> Result<(), KeyError> {
    // `symlink_metadata`: a dangling link at the path is something, and is never replaced.
    if path.symlink_metadata().is_ok() {
        return Ok(());
    }
    if !database_is_new {
        return Err(KeyError::Missing(path.display().to_string()));
    }
    let mut key = Zeroizing::new([0_u8; KEY_BYTES]);
    getrandom::fill(key.as_mut_slice()).map_err(|_| KeyError::Random)?;
    let text = Zeroizing::new(STANDARD.encode(key.as_slice()));
    write(path, text.as_bytes()).map_err(|error| KeyError::Write(error.kind()))
}

/// The whole key under a name beside `path`, then the name `path`: a crash never leaves half
/// a key under the name the hub reads.
fn write(path: &Path, text: &[u8]) -> io::Result<()> {
    let mut being_written = path.as_os_str().to_owned();
    being_written.push(".new");
    let being_written = Path::new(&being_written);
    let mut file = owner_only::new_file(being_written)?;
    file.write_all(text)?;
    file.sync_all()?;
    std::fs::rename(being_written, path)
}
