//! The programs the application carries beside its own executable (`anima-local-hub`,
//! `anima-link`): where they are, and how one runs (`run`).
//!
//! A release build finds them beside its executable only. A debug build carries none; to try
//! the app from `bun tauri dev`, `ANIMA_DEV_PROGRAMS` names a folder that holds them and
//! `ANIMA_DEV_TREE` names a PostgreSQL tree. A release build never reads either variable.

pub mod run;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

/// The folders a debug build was given for its carried programs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevFolders {
    pub programs: Option<PathBuf>,
    pub tree: Option<PathBuf>,
}

impl DevFolders {
    /// `ANIMA_DEV_PROGRAMS` and `ANIMA_DEV_TREE`, in a debug build only.
    #[cfg(debug_assertions)]
    pub fn of_this_run() -> Self {
        let read = |name| std::env::var_os(name).map(PathBuf::from);
        Self {
            programs: read("ANIMA_DEV_PROGRAMS"),
            tree: read("ANIMA_DEV_TREE"),
        }
    }

    /// A release build has none.
    #[cfg(not(debug_assertions))]
    pub fn of_this_run() -> Self {
        Self::default()
    }
}

/// The folder that holds the carried programs: beside `executable` in a release build; in a
/// debug build the absolute folder that `dev` names, or none.
pub fn programs_folder(executable: &Path, release: bool, dev: &DevFolders) -> Option<PathBuf> {
    if release {
        return executable.parent().map(Path::to_path_buf);
    }
    dev.programs
        .as_deref()
        .filter(|folder| folder.is_absolute())
        .map(Path::to_path_buf)
}

/// The carried program `name` of this build, when it is there.
pub fn program(name: &str) -> Option<PathBuf> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return None;
    }
    let executable = std::env::current_exe().ok()?;
    let folder = programs_folder(
        &executable,
        !cfg!(debug_assertions),
        &DevFolders::of_this_run(),
    )?;
    Some(folder.join(name)).filter(|program| program.is_file())
}
