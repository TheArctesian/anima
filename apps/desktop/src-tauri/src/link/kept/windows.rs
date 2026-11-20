//! Windows files used by the shared persistent-link lifecycle.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(super) fn environment(
    read: impl Fn(&str) -> Option<OsString>,
) -> Vec<(&'static str, OsString)> {
    [
        "SystemRoot",
        "WINDIR",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "TEMP",
        "TMP",
    ]
    .into_iter()
    .filter_map(|name| Some((name, read(name).filter(|value| !value.is_empty())?)))
    .collect()
}

pub(super) fn private_socket() -> Option<PathBuf> {
    let folder = crate::user_folders::of_this_user()?.join("daemon");
    anima_windows::files::directory(&folder).ok()?;
    let private = anima_windows::files::secure(&folder, true).ok()?;
    let socket = folder.join(super::DAEMON_SOCKET);
    (private && socket.as_os_str().len() <= super::LONGEST_SOCKET).then_some(socket)
}

pub(super) fn socket_present(socket: &Path) -> bool {
    anima_windows::socket::private(socket).unwrap_or(false)
}

/// Whether a Windows listener belongs to this user's protected persistent link executable.
pub fn owns_background(pid: u32) -> bool {
    let Some(application) = crate::user_folders::of_this_user() else {
        return false;
    };
    let program = crate::link::copy::destination(&application);
    anima_cli::mcp::install::setup::check_command(&program, 0).is_ok()
        && anima_windows::process::matches(pid, &program).unwrap_or(false)
}
