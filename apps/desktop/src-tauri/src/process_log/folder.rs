use std::{
    fs,
    io::ErrorKind,
    path::Path,
    process::{Command, Stdio},
};

use super::error::ProcessLogError;

/// The file-manager opener. An absolute path on macOS, so `PATH` cannot swap it.
#[cfg(target_os = "macos")]
pub(super) const OPENER: &str = "/usr/bin/open";
#[cfg(not(target_os = "macos"))]
pub(super) const OPENER: &str = "xdg-open";

/// Creates `folder` if it is missing, then runs `opener` with the folder as its one argument
/// (no shell) and waits for it. The opener's output is discarded: it can echo the path.
pub(super) fn open_with(folder: &Path, opener: &str) -> Result<(), ProcessLogError> {
    fs::create_dir_all(folder).map_err(|_| ProcessLogError::Failed)?;
    let status = Command::new(opener)
        .arg(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => ProcessLogError::Unavailable,
            _ => ProcessLogError::Failed,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(ProcessLogError::Failed)
    }
}
