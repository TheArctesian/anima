//! The folders on the operator's `PATH` (INS-004). An app opened from the Finder or a desktop menu
//! does not get the `PATH` of a terminal, so the app asks the operator's login shell, as a new
//! terminal would set it, and adds its own. The shell reads only its login profile (`-l`, not
//! interactive), gets no input and has 5 seconds.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

/// How long the login shell may take to print its `PATH`.
const PATIENCE: Duration = Duration::from_secs(5);
/// The start of the one line the shell prints, so profile output around it is ignored.
const MARK: &str = "anima-path:";

/// The `PATH` the shell printed after [`MARK`], from the last such line in `output`.
pub fn path_line(output: &str) -> Option<&str> {
    output
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(MARK))
}

/// The folders on the `PATH` of a terminal and of this process. A shell that fails or is slow
/// adds nothing.
pub async fn folders(shell: Option<&Path>, own: Option<OsString>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = own
        .as_deref()
        .map_or_else(Vec::new, |path| std::env::split_paths(path).collect());
    if let Some(shell) = shell
        && let Some(path) = login_path(shell).await
    {
        folders.extend(std::env::split_paths(&path));
    }
    folders
}

async fn login_path(shell: &Path) -> Option<String> {
    let mut command = tokio::process::Command::new(shell);
    command
        .args(["-l", "-c", &format!("printf '\\n{MARK}%s\\n' \"$PATH\"")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(PATIENCE, command.output())
        .await
        .ok()?
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    path_line(&text).map(str::to_owned)
}
