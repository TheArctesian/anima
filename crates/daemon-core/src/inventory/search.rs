//! Where a probe's program is looked for, and which found program may run. The daemon runs
//! under launchd or systemd with a short `PATH`, so it looks first in the folders that
//! developer tools install into, then in the `PATH`: a Homebrew or user tool wins over a
//! system one.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::Os;

/// The folders looked in before `PATH`, in order; `~` is the daemon's home folder.
pub const TOOL_FOLDERS: [&str; 9] = [
    "~/.cargo/bin",
    "/opt/homebrew/opt/rustup/bin",
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "~/.bun/bin",
    "~/.local/bin",
    "~/.docker/bin",
    "~/.nix-profile/bin",
    "/nix/var/nix/profiles/default/bin",
];

/// The programs in macOS's `/usr/bin` that only stand in for Apple's developer tools: without
/// the tools, running one opens the window that offers to install them.
const PLACEHOLDERS: [&str; 2] = ["git", "xcodebuild"];

/// Whether the daemon may run the program at `path`: not when, on macOS without the developer
/// tools, it is one of Apple's placeholders in `/usr/bin` (CAP-021).
#[must_use]
pub fn may_run(path: &Path, os: Os, developer_tools: bool) -> bool {
    let placeholder = os == Os::Macos
        && path.parent() == Some(Path::new("/usr/bin"))
        && path
            .file_name()
            .is_some_and(|name| PLACEHOLDERS.iter().any(|shim| OsStr::new(shim) == name));
    !placeholder || developer_tools
}

/// The folders to look in, in order and each once: `folders` with `~/` read as `home`, then
/// the absolute folders of `path` (a `PATH` value). A relative folder is skipped, so the
/// daemon's working folder is never searched; a `~` folder is skipped without a home.
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
    for folder in fixed.chain(listed) {
        if folder.is_absolute() && !found.contains(&folder) {
            found.push(folder);
        }
    }
    found
}
