//! Windows files used by the shared persistent-link lifecycle.
use crate::link::LinkError;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub(super) fn destination(home: &Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join("AppData/Local"))
        .join("Anima/bin/anima-link.exe")
}

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

pub(super) fn place(carried: &Path, home: &Path) -> Result<PathBuf, LinkError> {
    let destination = destination(home);
    if super::same(carried, &destination) {
        return Ok(destination);
    }
    let folder = destination.parent().ok_or(LinkError::Folder)?;
    anima_windows::files::directory(folder).map_err(|_| LinkError::Folder)?;
    anima_windows::files::protect(folder).map_err(|_| LinkError::Folder)?;
    let temporary = folder.join("anima-link.next.exe");
    // The destination folder has a checked private ACL; a different user cannot swap this file.
    fs::copy(carried, &temporary).map_err(|_| LinkError::Folder)?;
    anima_windows::files::protect(&temporary).map_err(|_| LinkError::Folder)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match fs::rename(&temporary, &destination) {
            Ok(()) => break,
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                let _ = fs::remove_file(&temporary);
                return Err(LinkError::Folder);
            }
        }
    }
    Ok(destination)
}

pub(super) fn private_socket() -> Option<PathBuf> {
    let folder = crate::user_folders::of_this_user()?.join("daemon");
    anima_windows::files::directory(&folder).ok()?;
    let private = anima_windows::files::secure(&folder, true).ok()?;
    let socket = folder.join(super::DAEMON_SOCKET);
    (private && socket.as_os_str().len() <= super::LONGEST_SOCKET).then_some(socket)
}
