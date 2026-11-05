use std::{
    fs,
    io::ErrorKind,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::error::ProcessLogError;

/// The file-manager opener. An absolute path on macOS, so `PATH` cannot swap it.
#[cfg(target_os = "macos")]
pub(super) const OPENER: &str = "/usr/bin/open";
#[cfg(not(target_os = "macos"))]
pub(super) const OPENER: &str = "xdg-open";

/// How long to wait for the opener's exit status. `open` and most `xdg-open` back ends hand
/// the folder to the file manager and exit at once; some `xdg-open` fallbacks run the file
/// manager in the foreground and exit only when it closes.
pub(super) const PATIENCE: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(20);

/// Creates `folder` if it is missing, then runs `opener` with the folder as its one argument
/// (no shell). The opener's output is discarded: it can echo the path.
///
/// An exit within `patience` decides the result. An opener still running after `patience` is
/// taken as a file manager in the foreground: the call succeeds and a thread reaps the child
/// when it exits, so a blocking `xdg-open` never leaves the command pending.
pub(super) fn open_with(
    folder: &Path,
    opener: &str,
    patience: Duration,
) -> Result<(), ProcessLogError> {
    fs::create_dir_all(folder).map_err(|_| ProcessLogError::Failed)?;
    let mut child = Command::new(opener)
        .arg(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => ProcessLogError::Unavailable,
            _ => ProcessLogError::Failed,
        })?;
    let deadline = Instant::now() + patience;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) | Err(_) => return Err(ProcessLogError::Failed),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) => {
                // The exit status no longer matters; waiting only reaps the child.
                thread::spawn(move || child.wait());
                return Ok(());
            }
        }
    }
}
