//! The files of a copy: the walk of the hub root that copies each entry, and the check of one
//! entry against the list.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use super::list::{Entry, Kind, List};
use super::{RootCopyError, io_error};
use crate::Folders;

/// What a copy leaves out: the programs (a hub of the pool runs them from its image, ROL-053),
/// and the two locks, which belong to the processes of the root.
const LEFT_OUT: [&str; 3] = ["programs", "lock", "change.lock"];

mod access;

/// Copies each entry of the root of `folders` to the new folder `to` and returns their list.
/// A socket is left out: it belongs to a process. A link is copied as a link.
pub(super) fn copy(folders: &Folders, to: &Path) -> Result<List, RootCopyError> {
    let root = folders.root();
    let cluster = folders.database().cluster();
    let mut list = List {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        cluster: cluster.strip_prefix(root).unwrap_or(&cluster).to_path_buf(),
        ..List::default()
    };
    fs::create_dir(to).map_err(io_error("make the folder of the files"))?;
    set_mode(to, 0o700)?;
    let mut waiting = vec![PathBuf::new()];
    while let Some(folder) = waiting.pop() {
        let mut names: Vec<_> = fs::read_dir(root.join(&folder))
            .and_then(Iterator::collect::<io::Result<Vec<_>>>)
            .map_err(io_error("read a folder of the hub root"))?
            .into_iter()
            .map(|entry| entry.file_name())
            .collect();
        names.sort();
        for name in names {
            if folder.as_os_str().is_empty() && LEFT_OUT.iter().any(|left| name == *left) {
                continue;
            }
            let path = folder.join(name);
            if let Some(entry) = copy_one(&root.join(&path), &to.join(&path), &path)? {
                if entry.kind == Kind::Folder {
                    waiting.push(path);
                }
                list.entries.push(entry);
            }
        }
    }
    list.entries.sort_by(|one, other| one.path.cmp(&other.path));
    Ok(list)
}

/// Copies one entry; `None` for a socket or a thing that is no file, folder or link.
fn copy_one(from: &Path, to: &Path, path: &Path) -> Result<Option<Entry>, RootCopyError> {
    let found = fs::symlink_metadata(from).map_err(io_error("look at a file of the hub root"))?;
    let mode = access::mode(&found);
    if access::socket(from, &found).map_err(io_error("inspect an endpoint of the hub root"))? {
        return Ok(None);
    }
    let kind = found.file_type();
    if kind.is_dir() {
        fs::create_dir(to).map_err(io_error("make a folder of the copy"))?;
        set_mode(to, mode)?;
        return Ok(Some(Entry {
            path: path.to_path_buf(),
            mode,
            kind: Kind::Folder,
        }));
    }
    if kind.is_symlink() {
        let target = fs::read_link(from).map_err(io_error("read a link of the hub root"))?;
        access::symlink(&target, to).map_err(io_error("make a link of the copy"))?;
        return Ok(Some(Entry {
            path: path.to_path_buf(),
            mode: 0,
            kind: Kind::Link { target },
        }));
    }
    if !kind.is_file() {
        return Ok(None);
    }
    let mut source = access::open(from).map_err(io_error("open a file of the hub root"))?;
    let mut new = access::create(to, mode).map_err(io_error("make a file of the copy"))?;
    let (bytes, sha256) = pass(&mut source, Some(&mut new)).map_err(io_error("copy a file"))?;
    new.sync_all()
        .map_err(io_error("write a file of the copy"))?;
    set_mode(to, mode)?;
    Ok(Some(Entry {
        path: path.to_path_buf(),
        mode,
        kind: Kind::File { bytes, sha256 },
    }))
}

fn set_mode(path: &Path, mode: u32) -> Result<(), RootCopyError> {
    access::set_mode(path, mode).map_err(io_error("set the mode of an entry of the copy"))
}

/// Reads `source` to its end, writes it to `to` when there is one, and returns the count of
/// the bytes and their SHA-256 in hex.
fn pass(source: &mut File, mut to: Option<&mut File>) -> io::Result<(u64, String)> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    let mut bytes = 0;
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        if let Some(to) = to.as_mut() {
            to.write_all(&buffer[..read])?;
        }
        bytes += read as u64;
    }
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        // Writing to a `String` does not fail.
        let _ = write!(hex, "{byte:02x}");
    }
    Ok((bytes, hex))
}

/// The entry of the file `file`, listed as `path`.
pub(super) fn file_entry(file: &Path, path: &Path) -> Result<Entry, RootCopyError> {
    let mut source = access::open(file).map_err(io_error("open a file of the copy"))?;
    let mode = access::mode(
        &source
            .metadata()
            .map_err(io_error("look at a file of the copy"))?,
    );
    let (bytes, sha256) = pass(&mut source, None).map_err(io_error("read a file of the copy"))?;
    Ok(Entry {
        path: path.to_path_buf(),
        mode,
        kind: Kind::File { bytes, sha256 },
    })
}

/// Checks that `entry` is below `folder` as the list says. The fault names the path only.
pub(super) fn verify(folder: &Path, entry: &Entry) -> Result<(), RootCopyError> {
    let fault = |what: &str| {
        Err(RootCopyError::Damaged(format!(
            "{}: {what}",
            entry.path.display()
        )))
    };
    let at = folder.join(&entry.path);
    let Ok(found) = fs::symlink_metadata(&at) else {
        return fault("is not there");
    };
    #[cfg(windows)]
    access::verify(&at).map_err(io_error("check a private entry of the copy"))?;
    let mode = access::mode(&found);
    let other_mode = !matches!(entry.kind, Kind::Link { .. }) && mode != entry.mode;
    match &entry.kind {
        Kind::Folder if !found.is_dir() => fault("is not a folder"),
        Kind::File { .. } if !found.is_file() => fault("is not a file"),
        Kind::Link { target } if fs::read_link(&at).ok().as_ref() != Some(target) => {
            fault("is not the link of the list")
        }
        _ if other_mode => fault("has a different mode"),
        Kind::Folder | Kind::Link { .. } => Ok(()),
        Kind::File { bytes, sha256 } => match file_entry(&at, &entry.path)?.kind {
            Kind::File { bytes: now, .. } if now != *bytes => fault("has a different size"),
            Kind::File { sha256: now, .. } if now != *sha256 => fault("has a different content"),
            _ => Ok(()),
        },
    }
}
