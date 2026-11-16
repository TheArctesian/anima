//! The file work of the carried step at the OS edge: the tree's new work folder, the signing
//! identity in the clone's `tauri.conf.json`, and the Mach-O files of the built app. The build
//! wrote every one of these places, so no link is followed and no path or text crosses in an
//! error (LOG-007).

use std::{
    fs::{self, DirBuilder},
    io::{self, Read},
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

use super::{mach_o, tree};
use crate::builder::{artifact, error::BuilderError};

/// The most of `tauri.conf.json` that is read.
const CONFIG_BYTES_MAX: u64 = 1 << 20;

/// Removes what is at `folder` and makes it anew, mode 0700, so the recipe trusts nothing an
/// earlier run left (D-1). A link at `folder` is removed as a link, never followed.
pub fn renew(folder: &Path) -> Result<(), BuilderError> {
    match fs::symlink_metadata(folder) {
        Ok(kind) if kind.is_dir() => fs::remove_dir_all(folder),
        Ok(_) => fs::remove_file(folder),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
    .map_err(|_| BuilderError::Bundle)?;
    DirBuilder::new()
        .mode(0o700)
        .create(folder)
        .map_err(|_| BuilderError::Bundle)
}

/// `bundle.macOS.signingIdentity` of the clone's own `tauri.conf.json`, the one Tauri signs with
/// (D-5, BLD-083); `Bundle` when the file is not the build's own or names none.
pub fn signing_identity(clone: &Path) -> Result<String, BuilderError> {
    let file = artifact::open_own(clone, &clone.join(tree::CONFIG))?;
    let config: serde_json::Value =
        serde_json::from_reader(file.take(CONFIG_BYTES_MAX)).map_err(|_| BuilderError::Bundle)?;
    tree::signing_identity(&config)
        .map(str::to_owned)
        .ok_or(BuilderError::Bundle)
}

/// Whether `name` is one a checked file may have: no control character, so no line `codesign`
/// prints with the path in it can look like another line.
fn plain(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| !name.chars().any(char::is_control))
}

/// Every regular file below `app` whose first bytes are a Mach-O magic (D-4). Links are neither
/// followed nor checked; a name with a control character fails.
pub fn mach_o_files(clone: &Path, app: &Path) -> Result<Vec<PathBuf>, BuilderError> {
    let mut folders = vec![app.to_owned()];
    let mut found = Vec::new();
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).map_err(|_| BuilderError::Bundle)? {
            let entry = entry.map_err(|_| BuilderError::Bundle)?;
            if !plain(&entry.file_name()) {
                return Err(BuilderError::Bundle);
            }
            let kind = entry.file_type().map_err(|_| BuilderError::Bundle)?;
            if kind.is_dir() {
                folders.push(entry.path());
            } else if kind.is_file() && mach_o::is_mach_o(&first_bytes(clone, &entry.path())?) {
                found.push(entry.path());
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The first four bytes of the build's regular file at `path`.
fn first_bytes(clone: &Path, path: &Path) -> Result<Vec<u8>, BuilderError> {
    let mut bytes = Vec::new();
    artifact::open_built(clone, path)?
        .take(4)
        .read_to_end(&mut bytes)
        .map_err(|_| BuilderError::Bundle)?;
    Ok(bytes)
}
