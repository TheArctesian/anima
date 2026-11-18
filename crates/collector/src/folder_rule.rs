//! The folders that one collector source may read (COL-006, AIS-008).

use std::path::{Component, Path, PathBuf};

/// The folder rule of a source. The default is all folders of the source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FolderRule {
    /// Each folder of the source.
    #[default]
    All,
    /// Each folder of the source but these and the folders in them.
    AllBut(Vec<PathBuf>),
    /// Only these folders and the folders in them.
    Only(Vec<PathBuf>),
}

impl FolderRule {
    /// Whether the source with the folder `root` may read `real`.
    ///
    /// `real` must be the path with each symbolic link resolved, so that a link that leaves
    /// the permitted folders is refused: the caller resolves it, and this compares whole path
    /// parts only. A path that is not absolute, or that has a `..` part, is not a resolved
    /// path and is refused. A listed folder outside `root` permits nothing.
    #[must_use]
    pub fn permits(&self, root: &Path, real: &Path) -> bool {
        if !is_resolved(real) || !real.starts_with(root) {
            return false;
        }
        let listed = |folders: &[PathBuf]| folders.iter().any(|folder| real.starts_with(folder));
        match self {
            Self::All => true,
            Self::AllBut(excluded) => !listed(excluded),
            Self::Only(included) => listed(included),
        }
    }
}

fn is_resolved(path: &Path) -> bool {
    path.is_absolute() && path.components().all(|part| part != Component::ParentDir)
}
