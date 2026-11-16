//! Where a probe's program is looked for. The agent runs under launchd or systemd with a short
//! `PATH`, so after the `PATH` it looks in the folders that developer tools install into.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The folders looked in after `PATH`, in order; `~` is the agent's home folder.
pub const TOOL_FOLDERS: [&str; 7] = [
    "~/.cargo/bin",
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "~/.bun/bin",
    "~/.local/bin",
    "~/.nix-profile/bin",
    "/nix/var/nix/profiles/default/bin",
];

/// The folders to look in, in order and each once: the absolute folders of `path` (a `PATH`
/// value), then `folders` with `~/` read as `home`. A relative folder is skipped, so the
/// agent's working folder is never searched; a `~` folder is skipped without a home.
#[must_use]
pub fn candidates(path: Option<&OsStr>, home: Option<&Path>, folders: &[&str]) -> Vec<PathBuf> {
    let listed = path.map(std::env::split_paths).into_iter().flatten();
    let fixed = folders
        .iter()
        .filter_map(|folder| match folder.strip_prefix("~/") {
            Some(rest) => home.map(|home| home.join(rest)),
            None => Some(PathBuf::from(folder)),
        });
    let mut found: Vec<PathBuf> = Vec::new();
    for folder in listed.chain(fixed) {
        if folder.is_absolute() && !found.contains(&folder) {
            found.push(folder);
        }
    }
    found
}
