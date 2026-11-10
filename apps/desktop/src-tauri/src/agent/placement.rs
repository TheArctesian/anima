//! Where the agent binary goes and how it gets there (MET-019, D-10 d): `~/.local/bin` on both
//! systems, the folder the page's install command uses, so either way of installing replaces the
//! same file and the unit or `LaunchAgent` that points at it stays right. The copy is atomic, mode
//! 0755, and refused when the folder or any folder above it can be written by others.

use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

/// The agent's file name, in the app bundle and in the destination folder. The release config
/// (`tauri.release.conf.json`) names the sidecar `binaries/anima-agent`; Tauri drops the target
/// triple and puts it beside the app's own executable.
pub const AGENT_FILE: &str = "anima-agent";

/// The installed binary's mode: the user runs it, everyone may read and run it.
const BINARY_MODE: u32 = 0o755;
/// A folder made on the way to the destination.
const FOLDER_MODE: u32 = 0o755;
/// No agent binary is this large; a larger file is not the agent.
const SIZE_LIMIT: u64 = 256 * 1024 * 1024;

const OTHERS_WRITE: u32 = 0o002;
const ROOT: u32 = 0;

/// Why the binary was not put in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    /// The destination folder, or one above it, is not owned by the user or root, or others may
    /// write to it.
    Untrusted,
    /// The bundled binary is not a regular file of a sane size.
    Source,
    /// A folder could not be made, or the file could not be written or renamed.
    Write,
}

/// `~/.local/bin/anima-agent` under `home`.
pub fn destination(home: &Path) -> PathBuf {
    home.join(".local/bin").join(AGENT_FILE)
}

/// The bundled agent beside `executable` (the app's own binary): Tauri's sidecar place.
pub fn bundled_beside(executable: &Path) -> Option<PathBuf> {
    executable.parent().map(|folder| folder.join(AGENT_FILE))
}

/// Whether a folder with `mode` and `owner` may hold, or lie above, the binary that the user's
/// unit starts at each login: owned by the user or root, and not writable by others. A sticky
/// folder (`/tmp`) is refused too: the agent's units hide `/tmp`, and no home lies under one.
pub fn folder_trusted(mode: u32, owner: u32, uid: u32) -> bool {
    (owner == uid || owner == ROOT) && mode & OTHERS_WRITE == 0
}

/// Makes the destination folder if needed, checks it and every folder above it, then copies
/// `source` beside the destination under a fresh name and renames it into place.
///
/// # Errors
/// [`PlacementError`] for the first step that fails; nothing is left at the destination then.
pub fn place(source: &Path, destination: &Path, uid: u32) -> Result<(), PlacementError> {
    let folder = destination.parent().ok_or(PlacementError::Write)?;
    DirBuilder::new()
        .recursive(true)
        .mode(FOLDER_MODE)
        .create(folder)
        .map_err(|_| PlacementError::Write)?;
    let folder = folder.canonicalize().map_err(|_| PlacementError::Write)?;
    check_chain(&folder, uid)?;
    let mut input = open_source(source)?;
    let part = folder.join(part_name()?);
    let copied = copy_to(&mut input, &part).and_then(|()| {
        fs::rename(&part, folder.join(AGENT_FILE))?;
        File::open(&folder)?.sync_all()
    });
    copied.map_err(|_| {
        // The part file may not exist (the create failed); the error that matters is the copy's.
        let _ = fs::remove_file(&part);
        PlacementError::Write
    })
}

/// Checks `folder` (canonical) and every folder above it.
fn check_chain(folder: &Path, uid: u32) -> Result<(), PlacementError> {
    for each in folder.ancestors() {
        let stat = fs::symlink_metadata(each).map_err(|_| PlacementError::Untrusted)?;
        if !stat.is_dir() || !folder_trusted(stat.mode(), stat.uid(), uid) {
            return Err(PlacementError::Untrusted);
        }
    }
    Ok(())
}

/// The bundled binary, opened; refused when it is a link, not a regular file, or too large.
fn open_source(source: &Path) -> Result<File, PlacementError> {
    let linked = fs::symlink_metadata(source).map_err(|_| PlacementError::Source)?;
    let file = File::open(source).map_err(|_| PlacementError::Source)?;
    let opened = file.metadata().map_err(|_| PlacementError::Source)?;
    let regular = linked.is_file() && opened.is_file() && linked.ino() == opened.ino();
    if !regular || opened.len() > SIZE_LIMIT {
        return Err(PlacementError::Source);
    }
    Ok(file)
}

/// A name no other file has: the create below refuses an existing name or a link.
fn part_name() -> Result<String, PlacementError> {
    let mut salt = [0_u8; 8];
    getrandom::fill(&mut salt).map_err(|_| PlacementError::Write)?;
    Ok(format!(
        ".{AGENT_FILE}.{:016x}.part",
        u64::from_le_bytes(salt)
    ))
}

/// Writes `input` to the new file `part` with the binary's mode, and flushes it to disk.
fn copy_to(input: &mut File, part: &Path) -> io::Result<()> {
    // `create_new` is O_CREAT|O_EXCL: an existing file or a planted link is refused, not followed.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(part)?;
    let copied = io::copy(&mut input.take(SIZE_LIMIT + 1), &mut output)?;
    if copied > SIZE_LIMIT {
        return Err(io::Error::other("the agent binary grew past the limit"));
    }
    output.flush()?;
    // Set after the write, so the umask does not decide the mode.
    output.set_permissions(fs::Permissions::from_mode(BINARY_MODE))?;
    output.sync_all()
}
