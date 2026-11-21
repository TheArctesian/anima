//! The unpack of a runtime archive (Section 6, F10): a `.tar.gz` ([`tar_gz`]) or a `.zip`
//! ([`zip`]), into a new empty folder, each file and folder made relative to an open handle of
//! its folder with `O_NOFOLLOW`. Only regular files and folders; no link, device or FIFO, no
//! absolute path, no `..`, no two names that are equal in lower case at any part of their
//! paths. Folders and program files get 0755, other files 0644, with no setuid, setgid or
//! sticky bit. At most 8 GiB of unpacked bytes and 50,000 entries, and never below the disk
//! reserve. Blocking work: the caller runs it on a blocking thread.

mod tar_gz;
mod zip;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::path::{Component, Path};

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::releases::ArchiveFormat;
use rustix::fs::{Mode, OFlags};

/// The limits of one unpack.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub bytes: u64,
    pub entries: usize,
}

impl Limits {
    /// The limits of Section 6: 8 GiB of unpacked bytes and 50,000 entries.
    pub const SECTION_6: Self = Self {
        bytes: 8 << 30,
        entries: 50_000,
    };
}

/// The cause of an archive that breaks a rule of the unpack (Section 5, R-NEW-archiverejected).
const BAD_ARCHIVE: Cause = Cause::ArchiveRejected;

const DIR_FLAGS: OFlags = OFlags::DIRECTORY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::RDONLY)
    .union(OFlags::CLOEXEC);

/// What one entry of an archive is, after the format read it.
#[derive(Clone, Copy)]
enum Kind {
    Folder,
    /// A regular file with the mode bits of the archive.
    File(u32),
}

fn open_dir(parent: &OwnedFd, name: &str, create: bool) -> io::Result<OwnedFd> {
    let made = create
        && match rustix::fs::mkdirat(parent, name, Mode::from_raw_mode(0o755)) {
            Ok(()) => true,
            Err(rustix::io::Errno::EXIST) => false,
            Err(error) => return Err(error.into()),
        };
    let dir = rustix::fs::openat(parent, name, DIR_FLAGS, Mode::empty())?;
    if made {
        // The mode of `mkdirat` passes through the umask; the contract wants 0755 exactly.
        rustix::fs::fchmod(&dir, Mode::from_raw_mode(0o755))?;
    }
    Ok(dir)
}

/// The names of `path`, when each part is a plain name of UTF-8 with no `\`.
fn names(path: &str) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for part in Path::new(path).components() {
        match part {
            Component::Normal(name) => {
                let name = name.to_str()?;
                if name.contains('\\') {
                    return None;
                }
                names.push(name.to_owned());
            }
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    (!names.is_empty()).then_some(names)
}

/// The writer of one unpack: the folder, the names seen, the bytes and entries left.
struct Writer<F: Fn() -> bool> {
    root: OwnedFd,
    /// Each path seen, and each of its parents, by its name in lower case.
    seen: HashMap<String, String>,
    left: u64,
    entries: usize,
    max_entries: usize,
    disk_holds: F,
}

impl<F: Fn() -> bool> Writer<F> {
    fn new(into: &Path, limits: Limits, disk_holds: F) -> Result<Self, Cause> {
        let root =
            rustix::fs::open(into, DIR_FLAGS, Mode::empty()).map_err(|_| Cause::LimitDisk)?;
        Ok(Self {
            root,
            seen: HashMap::new(),
            left: limits.bytes,
            entries: 0,
            max_entries: limits.entries,
            disk_holds,
        })
    }

    /// Two names equal in lower case at any part are refused: on a volume that ignores case
    /// they would be one folder or one file. A parent, or a folder entry, may come again with
    /// the same spelling.
    fn first_seen(&mut self, names: &[String], folder: bool) -> bool {
        for end in 1..=names.len() {
            let path = names[..end].join("/");
            let last = end == names.len();
            match self.seen.get(&path.to_lowercase()) {
                None => {
                    self.seen.insert(path.to_lowercase(), path);
                }
                Some(known) if *known == path && (!last || folder) => {}
                Some(_) => return false,
            }
        }
        true
    }

    /// Checks the limits before each entry.
    fn next_entry(&mut self) -> Result<(), Cause> {
        self.entries += 1;
        if self.entries > self.max_entries || !(self.disk_holds)() {
            return Err(Cause::LimitDisk);
        }
        Ok(())
    }

    /// Writes one entry at `path`.
    fn write(&mut self, path: &str, kind: Kind, bytes: &mut dyn Read) -> Result<(), Cause> {
        let names = names(path).ok_or(BAD_ARCHIVE)?;
        if !self.first_seen(&names, matches!(kind, Kind::Folder)) {
            return Err(BAD_ARCHIVE);
        }
        let (last, parents) = names.split_last().ok_or(BAD_ARCHIVE)?;
        let mut dir = rustix::fs::openat(&self.root, ".", DIR_FLAGS, Mode::empty())
            .map_err(|_| Cause::LimitDisk)?;
        for name in parents {
            dir = open_dir(&dir, name, true).map_err(|_| BAD_ARCHIVE)?;
        }
        match kind {
            Kind::Folder => drop(open_dir(&dir, last, true).map_err(|_| BAD_ARCHIVE)?),
            Kind::File(mode) => self.write_file(&dir, (last, mode), bytes)?,
        }
        Ok(())
    }

    fn write_file(
        &mut self,
        dir: &OwnedFd,
        (name, mode): (&str, u32),
        bytes: &mut dyn Read,
    ) -> Result<(), Cause> {
        let flags =
            OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mode = Mode::from_raw_mode(if mode & 0o111 != 0 { 0o755 } else { 0o644 });
        let fd = rustix::fs::openat(dir, name, flags, mode).map_err(|_| BAD_ARCHIVE)?;
        rustix::fs::fchmod(&fd, mode).map_err(|_| Cause::LimitDisk)?;
        let mut file = File::from(fd);
        let copied =
            io::copy(&mut bytes.take(self.left + 1), &mut file).map_err(|_| Cause::LimitDisk)?;
        if copied > self.left {
            return Err(Cause::LimitDisk);
        }
        self.left -= copied;
        Ok(())
    }
}

/// Unpacks `archive` of `format` into `into`, a new empty folder, within the limits of
/// Section 6. `disk_holds` says whether the free disk is still above the reserve; it is asked
/// before each entry.
///
/// # Errors
/// As [`unpack_within`].
pub fn unpack(
    archive: File,
    format: ArchiveFormat,
    into: &Path,
    disk_holds: impl Fn() -> bool,
) -> Result<(), Cause> {
    unpack_within(archive, (format, Limits::SECTION_6), into, disk_holds)
}

/// [`unpack`] with the limits `limits` (a test gives small ones).
///
/// # Errors
/// `archive_rejected` for an entry that breaks a rule; `limit_disk` past a limit, below the
/// reserve, or for a write that fails.
pub fn unpack_within(
    archive: File,
    (format, limits): (ArchiveFormat, Limits),
    into: &Path,
    disk_holds: impl Fn() -> bool,
) -> Result<(), Cause> {
    let mut writer = Writer::new(into, limits, disk_holds)?;
    match format {
        ArchiveFormat::TarGz => tar_gz::unpack(archive, &mut writer),
        ArchiveFormat::Zip => zip::unpack(archive, &mut writer),
    }
}
