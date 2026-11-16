//! Where the application carries the hub's programs. Pure but for two looks at the disk.

use std::path::{Path, PathBuf};

use super::LocalHubError;

const PROGRAM: &str = "anima-local-hub";

/// The program that does the work, and the PostgreSQL tree it copies at `install`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    pub program: PathBuf,
    pub tree: PathBuf,
}

/// The programs beside `executable`, in a release build only: a debug build's target folder
/// may hold programs built from another commit, which are never offered.
///
/// macOS: `Anima.app/Contents/MacOS/anima-local-hub` and `Contents/Resources/database/`.
/// Linux: `usr/bin/anima-local-hub` and `usr/lib/Anima/database/`.
pub fn beside(executable: &Path, macos: bool, release: bool) -> Result<Carried, LocalHubError> {
    if !release {
        return Err(LocalHubError::NotCarried);
    }
    let folder = executable.parent().ok_or(LocalHubError::NotCarried)?;
    let above = folder.parent().ok_or(LocalHubError::NotCarried)?;
    let carried = Carried {
        program: folder.join(PROGRAM),
        tree: if macos {
            above.join("Resources/database")
        } else {
            above.join("lib/Anima/database")
        },
    };
    let is_there = carried.program.is_file() && carried.tree.join("bin/postgres").is_file();
    if !is_there {
        return Err(LocalHubError::NotCarried);
    }
    Ok(carried)
}
