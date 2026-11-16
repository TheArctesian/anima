//! The file work of the carried step at the OS edge: the tree's new work folder, the signing
//! identity in the clone's `tauri.conf.json`, the files that must not be in the clone, and the
//! Mach-O files of the built app. The build wrote every one of these places, so no link is
//! followed and no path or text crosses in an error (LOG-007).

use std::{
    fs::{self, DirBuilder, Metadata},
    io::{self, Read},
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
};

use super::{mach_o, tree};
use crate::builder::{artifact, error::BuilderError};

/// The most of `tauri.conf.json` that is read.
const CONFIG_BYTES_MAX: u64 = 1 << 20;

/// Whether `kind` is a folder of this user that nobody else may write to.
fn private(kind: &Metadata) -> bool {
    kind.is_dir() && kind.uid() == nix::unistd::geteuid().as_raw() && kind.mode() & 0o022 == 0
}

/// Removes what is at `folder` and makes it anew, mode 0700, so the recipe trusts nothing an
/// earlier run left (D-1). The folder it is in must be this user's own, not writable by others,
/// and reached through no link, so the removal cannot be sent elsewhere. A link at `folder` is
/// removed as a link, never followed.
pub fn renew(folder: &Path) -> Result<(), BuilderError> {
    let parent = folder.parent().ok_or(BuilderError::Bundle)?;
    let real = fs::canonicalize(parent).map_err(|_| BuilderError::Bundle)?;
    let kind = fs::symlink_metadata(parent).map_err(|_| BuilderError::Bundle)?;
    if real != parent || !private(&kind) {
        return Err(BuilderError::Bundle);
    }
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
        .map_err(|_| BuilderError::Bundle)?;
    let made = fs::symlink_metadata(folder).map_err(|_| BuilderError::Bundle)?;
    (private(&made) && made.mode() & 0o777 == 0o700)
        .then_some(())
        .ok_or(BuilderError::Bundle)
}

/// Fails with `Dirty` when anything, a link too, is at `path`: a file git ignores would change
/// the build and never show in `git status`.
pub fn absent(path: &Path) -> Result<(), BuilderError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        _ => Err(BuilderError::Dirty),
    }
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
    let mut folders = vec![artifact::own_folder(clone, app)?];
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
