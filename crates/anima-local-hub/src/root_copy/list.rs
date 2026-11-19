//! The list of a copy of a hub root: one entry for each folder, file and link. Pure but for the
//! read and the write of the list file.

use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use anima_database::owner_only;
use serde::{Deserialize, Serialize};

use super::{RootCopyError, Summary, io_error};

/// The most bytes of a list: a hub root with a database has some thousands of entries.
const MOST_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Kind {
    Folder,
    File { bytes: u64, sha256: String },
    Link { target: PathBuf },
}

/// One entry: its path below the root of the copy, its mode bits and what it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub mode: u32,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    /// The version of the program that made the copy.
    pub version: String,
    pub made: u64,
    /// The folder of the database cluster below the root, for example `database/cluster`.
    pub cluster: PathBuf,
    pub entries: Vec<Entry>,
    pub dump: Option<Entry>,
}

impl List {
    pub(super) fn summary(&self) -> Summary {
        let bytes = self
            .entries
            .iter()
            .chain(&self.dump)
            .map(|entry| match entry.kind {
                Kind::File { bytes, .. } => bytes,
                Kind::Folder | Kind::Link { .. } => 0,
            });
        Summary {
            entries: self.entries.len(),
            bytes: bytes.sum(),
            made: self.made,
        }
    }

    /// The parts that a hub cannot start without: the version file of the cluster and the
    /// vault key. The other parts (sealing keys, files, link) are in the list when the root
    /// had them.
    pub(super) fn has_the_parts_of_a_hub(&self) -> Result<(), RootCopyError> {
        for part in [self.cluster.join("PG_VERSION"), PathBuf::from("vault.key")] {
            let listed = self
                .entries
                .iter()
                .any(|entry| entry.path == part && matches!(entry.kind, Kind::File { .. }));
            if !listed {
                return Err(RootCopyError::Damaged(format!(
                    "the list has no {}",
                    part.display()
                )));
            }
        }
        match self.entries.iter().find(|entry| !plain(&entry.path)) {
            Some(entry) => Err(RootCopyError::Damaged(format!(
                "the list names a path that leaves the copy: {}",
                entry.path.display()
            ))),
            None => Ok(()),
        }
    }

    pub(super) fn write(&self, file: &Path) -> Result<(), RootCopyError> {
        let text = serde_json::to_vec(self).map_err(|_| RootCopyError::NoList)?;
        let mut new = owner_only::new_file(file).map_err(io_error("make the list of the copy"))?;
        new.write_all(&text)
            .and_then(|()| new.sync_all())
            .map_err(io_error("write the list of the copy"))
    }

    pub(super) fn read(file: &Path) -> Result<Self, RootCopyError> {
        let size = fs::symlink_metadata(file).map_err(|_| RootCopyError::NoList)?;
        if !size.is_file() || size.len() > MOST_BYTES {
            return Err(RootCopyError::NoList);
        }
        let text = fs::read(file).map_err(|_| RootCopyError::NoList)?;
        serde_json::from_slice(&text).map_err(|_| RootCopyError::NoList)
    }
}

/// Whether `path` stays below the folder it is joined to: relative, with only plain names.
fn plain(path: &Path) -> bool {
    path.components().next().is_some()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
