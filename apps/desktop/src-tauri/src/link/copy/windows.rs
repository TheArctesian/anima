//! Protected persistent-link placement on Windows.
use crate::link::LinkError;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn destination(application: &Path) -> PathBuf {
    application.join("bin/anima-link.exe")
}

pub fn earlier_copy(home: &Path) -> PathBuf {
    let application =
        crate::user_folders::of_this_user().unwrap_or_else(|| home.join("AppData/Local/Anima"));
    destination(&application)
}

pub fn current_copy(application: &Path, _home: &Path) -> PathBuf {
    destination(application)
}

pub fn same(one: &Path, other: &Path) -> bool {
    match (fs::read(one), fs::read(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

pub fn unplace(program: &Path) {
    let _ = fs::remove_file(program);
}

pub fn place(carried: &Path, application: &Path) -> Result<PathBuf, LinkError> {
    let destination = destination(application);
    if same(carried, &destination) {
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

/// Windows replaces no running copy (the unit is removed first), so no copy waits.
pub fn waits_for_restart(_program: &Path) -> bool {
    false
}

/// Nothing to delete on Windows: see [`waits_for_restart`].
pub fn forget_previous(_program: &Path) {}
