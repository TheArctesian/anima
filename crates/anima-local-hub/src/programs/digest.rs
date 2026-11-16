//! The digest of a carried set: what the update compares to see whether the copy that the
//! unit runs is the copy of the programs that the application carries (D-6).
//!
//! The digest is a SHA-256 over every entry in the layout of the installed copy (`bin/` and
//! `database-tree/`), in the order of the entries' relative names, so that the source
//! folders and a faithful copy give the same digest. Each entry gives its kind, its name, and
//! for a file whether it runs, its length and its bytes, and for a link its target. The
//! refusals and the limits are those of the copy.

use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Read as _;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

#[cfg(test)]
mod tests;

use super::{Budget, CopyError, MOST_BYTES, MOST_DEPTH, MOST_ENTRIES, io_error, link_target};
use crate::Folders;
use crate::members::{Carried, PROGRAMS};

/// The file in `programs/current` that holds the digest of the copy that `copy` made.
pub(super) const FILE: &str = "digest";

/// The hex form of a SHA-256: 64 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest(String);

impl Digest {
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0
    }

    /// Whether the copy in `programs/current` has this digest. A copy that an older version
    /// made has no digest file, and then it differs.
    #[must_use]
    pub fn is_stored_in(&self, folders: &Folders) -> bool {
        crate::plain_file::read_text(&folders.programs().join(FILE), 128)
            .is_ok_and(|text| text.trim() == self.0)
    }
}

enum Kind {
    Folder,
    /// A program always runs: the copy makes it 0700.
    File {
        program: bool,
    },
    Link(PathBuf),
}

struct Entry {
    /// The name in the installed layout.
    name: PathBuf,
    /// Where the entry is now.
    from: PathBuf,
    kind: Kind,
}

/// The digest of `carried`, which is either what the application carries or a copy.
///
/// # Errors
/// As [`super::copy`] for the same set: a program that is missing, an entry that is not a
/// file, a folder or a link that stays beside its own folder, a set larger than a tree can
/// be, or an entry that cannot be read.
pub fn of(carried: &Carried) -> Result<Digest, CopyError> {
    let layout = Carried::installed_in(Path::new(""));
    let mut walk = Walk {
        entries: vec![Entry {
            name: layout.programs().to_path_buf(),
            from: carried.programs().to_path_buf(),
            kind: Kind::Folder,
        }],
        budget: Budget {
            entries: MOST_ENTRIES,
            bytes: MOST_BYTES,
        },
    };
    for name in PROGRAMS {
        let from = carried.program(name);
        if !from.symlink_metadata().is_ok_and(|found| found.is_file()) {
            return Err(CopyError::NotCarried(from));
        }
        walk.entries.push(Entry {
            name: layout.program(name),
            from,
            kind: Kind::File { program: true },
        });
    }
    walk.folder(carried.tree(), layout.tree(), MOST_DEPTH)?;
    walk.hash()
}

struct Walk {
    entries: Vec<Entry>,
    budget: Budget,
}

impl Walk {
    fn folder(&mut self, from: &Path, name: &Path, depth: usize) -> Result<(), CopyError> {
        if depth == 0 {
            return Err(CopyError::TooLarge);
        }
        let listing = fs::read_dir(from).map_err(io_error("read", from))?;
        self.entries.push(Entry {
            name: name.to_path_buf(),
            from: from.to_path_buf(),
            kind: Kind::Folder,
        });
        for entry in listing {
            let entry = entry.map_err(io_error("read", from))?;
            self.budget.entries = self
                .budget
                .entries
                .checked_sub(1)
                .ok_or(CopyError::TooLarge)?;
            let (path, name) = (entry.path(), name.join(entry.file_name()));
            let kind = entry.file_type().map_err(io_error("read", &path))?;
            if kind.is_dir() {
                self.folder(&path, &name, depth - 1)?;
            } else if kind.is_file() {
                self.entries.push(Entry {
                    name,
                    from: path,
                    kind: Kind::File { program: false },
                });
            } else if kind.is_symlink() {
                let target = link_target(&path)?;
                self.entries.push(Entry {
                    name,
                    from: path,
                    kind: Kind::Link(target),
                });
            } else {
                return Err(CopyError::Refused(path));
            }
        }
        Ok(())
    }

    fn hash(mut self) -> Result<Digest, CopyError> {
        self.entries
            .sort_by(|one, other| one.name.as_os_str().cmp(other.name.as_os_str()));
        let mut hasher = Sha256::new();
        for entry in &self.entries {
            hasher.update([match entry.kind {
                Kind::Folder => b'd',
                Kind::File { .. } => b'f',
                Kind::Link(_) => b'l',
            }]);
            text(&mut hasher, entry.name.as_os_str().as_bytes());
            match &entry.kind {
                Kind::Folder => {}
                Kind::Link(target) => text(&mut hasher, target.as_os_str().as_bytes()),
                Kind::File { program } => {
                    file(&mut hasher, &entry.from, *program, &mut self.budget)?;
                }
            }
        }
        let hex = hasher
            .finalize()
            .iter()
            .fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            });
        Ok(Digest(hex))
    }
}

/// Bytes with their length first, so that no two lists of entries give the same input.
fn text(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn file(
    hasher: &mut Sha256,
    from: &Path,
    program: bool,
    budget: &mut Budget,
) -> Result<(), CopyError> {
    use nix::fcntl::OFlag;
    // No link is followed, and a pipe that took the file's place does not hold the bytes.
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags((OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK).bits())
        .open(from)
        .map_err(io_error("read", from))?;
    let found = source.metadata().map_err(io_error("read", from))?;
    if !found.is_file() {
        return Err(CopyError::Refused(from.to_path_buf()));
    }
    budget.bytes = budget
        .bytes
        .checked_sub(found.len())
        .ok_or(CopyError::TooLarge)?;
    let runs = program || found.permissions().mode() & 0o111 != 0;
    hasher.update([u8::from(runs)]);
    hasher.update(found.len().to_be_bytes());
    // No more than the size that was counted, also when the file grows meanwhile.
    let mut left = found.len();
    let mut chunk = [0_u8; 8 * 1024];
    while left > 0 {
        let size = usize::try_from(left).map_or(chunk.len(), |left| left.min(chunk.len()));
        source
            .read_exact(&mut chunk[..size])
            .map_err(io_error("read", from))?;
        hasher.update(&chunk[..size]);
        left -= size as u64;
    }
    Ok(())
}

/// The digest file of the copy that `copy` made at `next`, in the mode of the copy's files.
pub(super) fn write(next: &Path, digest: &Digest) -> Result<(), CopyError> {
    use std::io::Write as _;
    let path = next.join(FILE);
    let mut file = anima_database::owner_only::new_file(&path).map_err(io_error("write", &path))?;
    writeln!(file, "{}", digest.hex())
        .and_then(|()| file.sync_all())
        .map_err(io_error("write", &path))
}
