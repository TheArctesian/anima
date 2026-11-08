use std::{fs, path::Path, time::Duration};

use super::error::ProcessLogError;
use crate::os_program::{self, OsProgramError};

/// The file-manager opener. An absolute path on macOS, so `PATH` cannot swap it.
#[cfg(target_os = "macos")]
pub(super) const OPENER: &str = "/usr/bin/open";
#[cfg(not(target_os = "macos"))]
pub(super) const OPENER: &str = "xdg-open";

/// Creates `folder` if it is missing, then runs `opener` with the folder as its one argument
/// (no shell). The opener's output is discarded: it can echo the path. An opener still running
/// after `patience` is taken as a file manager in the foreground (see
/// `os_program::open_within`).
pub(super) fn open_with(
    folder: &Path,
    opener: &str,
    patience: Duration,
) -> Result<(), ProcessLogError> {
    fs::create_dir_all(folder).map_err(|_| ProcessLogError::Failed)?;
    os_program::open_within(opener, folder.as_os_str(), patience).map_err(|error| match error {
        OsProgramError::NotFound => ProcessLogError::Unavailable,
        _ => ProcessLogError::Failed,
    })
}
