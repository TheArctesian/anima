//! Where the agent binary goes and how it gets there (MET-019, D-10 d): `~/.local/bin` on both
//! systems, the folder the page's install command uses, so either way of installing replaces the
//! same file and the unit or `LaunchAgent` that points at it stays right. The copy is atomic,
//! mode 0755, and refused when the folder or any folder above it can be changed by another user
//! ([`super::trust`]).

use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use super::trust::{self, Stat, User};

/// The agent's file name, in the app bundle and in the destination folder. The release config
/// (`tauri.release.conf.json`) names the sidecar `binaries/anima-agent`; Tauri drops the target
/// triple and puts it beside the app's own executable.
pub const AGENT_FILE: &str = "anima-agent";

/// The installed binary's mode: the user runs it, everyone may read and run it.
const BINARY_MODE: u32 = 0o755;
/// A folder made on the way to the destination: public, and only its owner may change it.
const FOLDER_MODE: u32 = 0o755;
/// No agent binary is this large; a larger file is not the agent.
const SIZE_LIMIT: u64 = 256 * 1024 * 1024;

/// Why the binary was not put in place. Fixed text: no path, no OS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlacementError {
    /// A folder on the way is not the user's or root's, or another user may write to it.
    #[error("a folder on the way to the agent can be changed by another user")]
    Untrusted,
    #[error("the bundled agent is not a regular file of a sane size")]
    Source,
    #[error("a folder could not be made, or the agent could not be written or renamed")]
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

/// Makes the destination folder through checked folders only, then copies `source` beside the
/// destination under a fresh name and renames it into place. Returns the real path of the
/// installed binary: the path that was checked, and the one to run.
///
/// # Errors
/// [`PlacementError`] for the first step that fails; nothing is left at the destination then.
pub fn place(source: &Path, destination: &Path, by: User) -> Result<PathBuf, PlacementError> {
    let folder = make_folder(destination.parent().ok_or(PlacementError::Write)?, by)?;
    let mut input = open_source(source)?;
    let part = folder.join(part_name()?);
    let installed = folder.join(AGENT_FILE);
    let copied = copy_to(&mut input, &part).and_then(|()| {
        fs::rename(&part, &installed)?;
        File::open(&folder)?.sync_all()
    });
    copied.map_err(|_| {
        // The part file may not exist (the create failed); the error that matters is the copy's.
        let _ = fs::remove_file(&part);
        PlacementError::Write
    })?;
    Ok(installed)
}

/// Makes `dir` and returns its real path, as T6a's `make_folder` does: the deepest part that
/// exists is resolved and checked with every folder above it first, then each missing part is
/// made in that checked folder, resolved and checked in turn. No folder is made through a link
/// to a folder another user may write.
fn make_folder(dir: &Path, by: User) -> Result<PathBuf, PlacementError> {
    let existing = dir
        .ancestors()
        .find(|folder| fs::metadata(folder).is_ok())
        .unwrap_or_else(|| Path::new("/"));
    let mut real = existing.canonicalize().map_err(|_| PlacementError::Write)?;
    check_chain(&real, by)?;
    let missing = dir.strip_prefix(existing).unwrap_or_else(|_| Path::new(""));
    for part in missing.components() {
        real.push(part);
        match DirBuilder::new().mode(FOLDER_MODE).create(&real) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
                return Err(PlacementError::Write);
            }
            _ => {}
        }
        real = real.canonicalize().map_err(|_| PlacementError::Write)?;
        check_chain(&real, by)?;
    }
    Ok(real)
}

/// Checks `folder` (a real path) and every folder above it.
fn check_chain(folder: &Path, by: User) -> Result<(), PlacementError> {
    for each in folder.ancestors() {
        let metadata = fs::symlink_metadata(each).map_err(|_| PlacementError::Untrusted)?;
        let stat = Stat {
            mode: metadata.mode(),
            owner: metadata.uid(),
            group: metadata.gid(),
            acl: has_acl(each)?,
        };
        if !metadata.is_dir() || !trust::folder_trusted(stat, by) {
            return Err(PlacementError::Untrusted);
        }
    }
    Ok(())
}

/// Whether `path` carries a POSIX access ACL (`system.posix_acl_access`); a file system without
/// ACLs has none. One that cannot be read is not trusted.
#[cfg(target_os = "linux")]
fn has_acl(path: &Path) -> Result<bool, PlacementError> {
    use rustix::io::Errno;
    match rustix::fs::getxattr_query(path, "system.posix_acl_access") {
        Ok(_) => Ok(true),
        Err(Errno::NODATA | Errno::NOTSUP) => Ok(false),
        Err(_) => Err(PlacementError::Untrusted),
    }
}

/// macOS ACLs (`chmod +a`) do not show in the mode and are not read here; T6a has the same limit.
#[cfg(not(target_os = "linux"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as the Linux reader"
)]
fn has_acl(_path: &Path) -> Result<bool, PlacementError> {
    Ok(false)
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
