//! The folders a program is looked for in: `PATH` first, then the fixed folders, each once, and
//! never a relative one (CAP-021).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::inventory::{TOOL_FOLDERS, candidates};

fn paths(texts: &[&str]) -> Vec<PathBuf> {
    texts.iter().map(PathBuf::from).collect()
}

#[test]
fn cap_021_path_comes_first_then_the_fixed_folders_with_the_home_folder() {
    let found = candidates(
        Some(OsStr::new("/usr/bin:/bin")),
        Some(Path::new("/Users/op")),
        &TOOL_FOLDERS,
    );
    assert_eq!(
        found,
        paths(&[
            "/usr/bin",
            "/bin",
            "/Users/op/.cargo/bin",
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/Users/op/.bun/bin",
            "/Users/op/.local/bin",
            "/Users/op/.nix-profile/bin",
            "/nix/var/nix/profiles/default/bin",
        ])
    );
}

#[test]
fn cap_021_a_relative_or_repeated_folder_is_skipped_and_no_home_skips_the_home_folders() {
    let found = candidates(
        Some(OsStr::new(".:bin::/usr/local/bin:/usr/bin")),
        None,
        &["~/.cargo/bin", "/usr/local/bin", "relative"],
    );
    assert_eq!(found, paths(&["/usr/local/bin", "/usr/bin"]));
    let relative_home = candidates(None, Some(Path::new("home")), &["~/.cargo/bin"]);
    assert_eq!(relative_home, Vec::<PathBuf>::new());
}
