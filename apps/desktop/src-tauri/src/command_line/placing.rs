//! Putting the link or launcher at `<folder>/anima`, and taking it away (INS-004 to INS-006). A new
//! one never replaces anything: the link or hard link fails when a file appeared meanwhile. Only an
//! Anima app's own entry is replaced, by a rename from a fresh name in the same folder.

use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use super::{carrier::Carrier, launcher};

/// The launcher's mode: the user runs it, everyone may read and run it (INS-005).
const LAUNCHER_MODE: u32 = 0o755;
/// A folder the app makes (`~/.local/bin`): public, and only its owner may change it.
const FOLDER_MODE: u32 = 0o755;

/// Makes `folder` and any folder above it that is missing.
pub fn make_folder(folder: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(FOLDER_MODE)
        .create(folder)
}

/// Puts this app's link or launcher at `destination`. With `replace`, an entry already there is
/// replaced (the caller checked it is an Anima app's own); without, an entry there fails the put.
pub fn put(destination: &Path, carrier: &Carrier, replace: bool) -> io::Result<()> {
    let folder = destination
        .parent()
        .ok_or_else(|| io::Error::other("no folder"))?;
    let part = folder.join(part_name()?);
    let made = match carrier {
        Carrier::Bundle { program } => symlink(program, &part),
        Carrier::AppImage { file } => write_launcher(&part, file),
    };
    let placed = made.and_then(|()| {
        if replace {
            fs::rename(&part, destination)
        } else {
            // A hard link of the new entry: refused when the name exists, unlike a rename.
            link_new(&part, destination).and_then(|()| fs::remove_file(&part))
        }
    });
    if placed.is_err() {
        let _ = fs::remove_file(&part);
    }
    placed?;
    File::open(folder)?.sync_all()
}

/// `destination` as a second name of `part`. For a link, a new link with the same target: a hard
/// link to a symbolic link is not portable.
fn link_new(part: &Path, destination: &Path) -> io::Result<()> {
    if fs::symlink_metadata(part)?.file_type().is_symlink() {
        symlink(fs::read_link(part)?, destination)
    } else {
        fs::hard_link(part, destination)
    }
}

/// Removes the entry at `destination` itself; a link's target is not touched.
pub fn take_away(destination: &Path) -> io::Result<()> {
    fs::remove_file(destination)
}

/// Writes the launcher for `appimage` to the new file `part`, mode 0755, flushed to disk.
fn write_launcher(part: &Path, appimage: &Path) -> io::Result<()> {
    let text =
        launcher::script(appimage).ok_or_else(|| io::Error::other("unquotable AppImage path"))?;
    // `create_new` is O_CREAT|O_EXCL: an existing file or a planted link is refused, not followed.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(part)?;
    output.write_all(text.as_bytes())?;
    // Set after the write, so the umask does not decide the mode.
    output.set_permissions(fs::Permissions::from_mode(LAUNCHER_MODE))?;
    output.sync_all()
}

/// A name no other file in the folder has.
fn part_name() -> io::Result<PathBuf> {
    let mut salt = [0_u8; 8];
    getrandom::fill(&mut salt).map_err(|_| io::Error::other("no random bytes"))?;
    Ok(PathBuf::from(format!(
        ".anima.{:016x}.part",
        u64::from_le_bytes(salt)
    )))
}
