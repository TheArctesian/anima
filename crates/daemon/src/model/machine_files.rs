//! The two files of model work in the daemon folder (Sections 6 and 10.1): `model-work.json`,
//! the settings at the machine, and `model-sources.json`, the release sources. Only the owner of
//! the machine writes them; the hub cannot. A file that is not there has the defaults.

use std::io;
use std::path::Path;

use daemon_core::model_work::machine::{MachineSettings, SettingsError};
use daemon_core::model_work::releases::Sources;

pub const SETTINGS_FILE: &str = "model-work.json";
pub const SOURCES_FILE: &str = "model-sources.json";

/// The most bytes of each file.
const MAX_BYTES: u64 = 256 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum MachineFileError {
    #[error("a file of model work: {0}")]
    Io(#[from] io::Error),
    #[error("a file of model work is a link, not a regular file, or too large")]
    NotPlain,
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error("model-sources.json does not read")]
    Sources,
}

/// The text of `name` in `home`, or `None` when it is not there. No link is followed.
fn read(home: &Path, name: &str) -> Result<Option<String>, MachineFileError> {
    let path = home.join(name);
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(MachineFileError::NotPlain);
    }
    Ok(Some(std::fs::read_to_string(path)?))
}

/// The settings at the machine.
///
/// # Errors
/// [`MachineFileError`] for a file that is there and does not read; the daemon then takes no
/// model item.
pub fn settings(home: &Path) -> Result<MachineSettings, MachineFileError> {
    match read(home, SETTINGS_FILE)? {
        Some(text) => Ok(MachineSettings::parse(&text)?),
        None => Ok(MachineSettings::default()),
    }
}

/// The release sources.
///
/// # Errors
/// [`MachineFileError`] for a file that is there and does not read.
pub fn sources(home: &Path) -> Result<Sources, MachineFileError> {
    match read(home, SOURCES_FILE)? {
        Some(text) => serde_json::from_str(&text).map_err(|_| MachineFileError::Sources),
        None => Ok(Sources::default()),
    }
}
