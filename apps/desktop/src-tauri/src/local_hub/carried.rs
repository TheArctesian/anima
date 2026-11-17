//! Where the application carries the hub's programs. Pure but for two looks at the disk.

use std::path::{Path, PathBuf};

use super::LocalHubError;
use crate::carried::{DevFolders, programs_folder};

const PROGRAM: &str = "anima-local-hub";
/// The tree below `usr/` of the Linux bundle: Tauri puts resources in `lib/<productName>/`.
pub(super) const LINUX_TREE: &str = "lib/Anima.ing/database";

/// The program that does the work, and the PostgreSQL tree it copies at `install`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    pub program: PathBuf,
    pub tree: PathBuf,
}

/// The programs beside `executable`, in a release build: a debug build's target folder may
/// hold programs built from another commit, which are never offered. A debug build uses the
/// folders of `dev`, when it has both.
///
/// macOS: `Anima.ing.app/Contents/MacOS/anima-local-hub` and `Contents/Resources/database/`.
/// Linux: `usr/bin/anima-local-hub` and `usr/lib/Anima.ing/database/`.
pub fn beside(
    executable: &Path,
    macos: bool,
    release: bool,
    dev: &DevFolders,
) -> Result<Carried, LocalHubError> {
    let folder = programs_folder(executable, release, dev).ok_or(LocalHubError::NotCarried)?;
    let tree = if release {
        let above = folder.parent().ok_or(LocalHubError::NotCarried)?;
        if macos {
            above.join("Resources/database")
        } else {
            above.join(LINUX_TREE)
        }
    } else {
        dev.tree
            .clone()
            .filter(|tree| tree.is_absolute())
            .ok_or(LocalHubError::NotCarried)?
    };
    let carried = Carried {
        program: folder.join(PROGRAM),
        tree,
    };
    let is_there = carried.program.is_file() && carried.tree.join("bin/postgres").is_file();
    if !is_there {
        return Err(LocalHubError::NotCarried);
    }
    Ok(carried)
}
