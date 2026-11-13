//! Which folder gets the operator's `anima` (INS-004, INS-005, INS-009). The app never asks for an
//! administrator password, so it offers only a folder this user can write: the first of a short
//! list that exists, is writable and is on the `PATH`. When none is, it offers `~/.local/bin`,
//! makes it, and shows the line to add to the shell profile; it never edits the profile. Pure: the
//! `PATH` folders and the writable check are parameters.

use std::path::{Path, PathBuf};

/// The line that puts `~/.local/bin` on the `PATH`, for the operator to add to the profile.
pub const PROFILE_LINE: &str = r#"export PATH="$HOME/.local/bin:$PATH""#;

/// The folders the app may use, in the order it prefers them. `/usr/local/bin` only when this
/// user can write it, which the writable check decides.
pub fn candidates(home: &Path) -> [PathBuf; 3] {
    [
        home.join(".local/bin"),
        home.join("bin"),
        PathBuf::from("/usr/local/bin"),
    ]
}

/// The folder the app proposes, and what the operator must know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub folder: PathBuf,
    /// Whether the folder is on the operator's `PATH`; if not, show [`PROFILE_LINE`].
    pub on_path: bool,
    /// Whether the app must make the folder first.
    pub create: bool,
}

/// The first candidate under `home` that `writable` accepts and that is in `path`; else
/// `~/.local/bin`. `writable` answers for an existing folder this user may write.
pub fn propose(home: &Path, path: &[PathBuf], writable: impl Fn(&Path) -> bool) -> Proposal {
    let on_path = |folder: &Path| path.iter().any(|entry| entry == folder);
    let [fallback, ..] = candidates(home);
    if let Some(folder) = candidates(home)
        .into_iter()
        .find(|folder| writable(folder) && on_path(folder))
    {
        return Proposal {
            folder,
            on_path: true,
            create: false,
        };
    }
    Proposal {
        on_path: on_path(&fallback),
        create: !writable(&fallback),
        folder: fallback,
    }
}
