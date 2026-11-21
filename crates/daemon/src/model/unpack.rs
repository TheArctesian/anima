//! The unpack of a runtime archive (Section 6, F10): a `.tar.gz`, into a new empty folder, each
//! file and folder made relative to an open handle of its folder with `O_NOFOLLOW`. Only
//! regular files and folders; no link, device or FIFO, no absolute path, no `..`, no two names
//! equal in lower case. Folders and program files get 0755, other files 0644, with no setuid,
//! setgid or sticky bit. At most 8 GiB of unpacked bytes and 50,000 entries.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::path::{Component, Path};

use daemon_core::model_work::cause::Cause;
use flate2::read::GzDecoder;
use rustix::fs::{Mode, OFlags};
use tar::EntryType;

pub const MAX_BYTES: u64 = 8 << 30;
pub const MAX_ENTRIES: usize = 50_000;

/// The cause of an archive that breaks a rule of the unpack. Its SHA-256 was the one of the
/// configuration, so the source holds a release that this program does not take.
const BAD_ARCHIVE: Cause = Cause::ProgramMismatch;

fn open_dir(parent: &OwnedFd, name: &str, create: bool) -> io::Result<OwnedFd> {
    if create {
        match rustix::fs::mkdirat(parent, name, Mode::from_raw_mode(0o755)) {
            Ok(()) | Err(rustix::io::Errno::EXIST) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
    Ok(rustix::fs::openat(parent, name, flags, Mode::empty())?)
}

/// The names of the path, when each part is a plain name.
fn names(path: &Path) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(name) => names.push(name.to_str()?.to_owned()),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    (!names.is_empty()).then_some(names)
}

fn write_file<R: Read>(
    dir: &OwnedFd,
    name: &str,
    entry: &mut tar::Entry<'_, R>,
    left: &mut u64,
) -> Result<(), Cause> {
    let mode = entry.header().mode().map_err(|_| BAD_ARCHIVE)?;
    let flags = OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let program = mode & 0o111 != 0;
    let mode = Mode::from_raw_mode(if program { 0o755 } else { 0o644 });
    let fd = rustix::fs::openat(dir, name, flags, mode).map_err(|_| BAD_ARCHIVE)?;
    rustix::fs::fchmod(&fd, mode).map_err(|_| Cause::LimitDisk)?;
    let mut file = File::from(fd);
    let copied =
        io::copy(&mut entry.by_ref().take(*left + 1), &mut file).map_err(|_| Cause::LimitDisk)?;
    if copied > *left {
        return Err(Cause::LimitDisk);
    }
    *left -= copied;
    Ok(())
}

/// Unpacks `archive` into `into`, a new empty folder.
///
/// # Errors
/// `program_mismatch` for an entry that breaks a rule; `limit_disk` past a limit or for a
/// write that fails.
pub fn unpack(archive: File, into: &Path) -> Result<(), Cause> {
    let flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
    let root = rustix::fs::open(into, flags, Mode::empty()).map_err(|_| Cause::LimitDisk)?;
    let mut tar = tar::Archive::new(GzDecoder::new(archive));
    let mut seen = HashSet::new();
    let mut left = MAX_BYTES;
    for (count, entry) in tar.entries().map_err(|_| BAD_ARCHIVE)?.enumerate() {
        if count >= MAX_ENTRIES {
            return Err(Cause::LimitDisk);
        }
        let mut entry = entry.map_err(|_| BAD_ARCHIVE)?;
        let kind = entry.header().entry_type();
        let path = entry.path().map_err(|_| BAD_ARCHIVE)?.into_owned();
        let names = names(&path).ok_or(BAD_ARCHIVE)?;
        if !seen.insert(names.join("/").to_lowercase()) {
            return Err(BAD_ARCHIVE);
        }
        let (last, parents) = names.split_last().ok_or(BAD_ARCHIVE)?;
        let mut dir = open_dir(&root, ".", false).map_err(|_| Cause::LimitDisk)?;
        for name in parents {
            dir = open_dir(&dir, name, true).map_err(|_| BAD_ARCHIVE)?;
        }
        match kind {
            EntryType::Directory => drop(open_dir(&dir, last, true).map_err(|_| BAD_ARCHIVE)?),
            EntryType::Regular | EntryType::Continuous => {
                write_file(&dir, last, &mut entry, &mut left)?;
            }
            _ => return Err(BAD_ARCHIVE),
        }
    }
    Ok(())
}
