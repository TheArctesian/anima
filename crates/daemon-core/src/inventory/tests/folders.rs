//! The folders a program is looked for in: the fixed folders first, then `PATH`, each once, and
//! never a relative one; and Apple's placeholders that must not run (CAP-021).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::inventory::{Os, TOOL_FOLDERS, candidates, may_run};

fn paths(texts: &[&str]) -> Vec<PathBuf> {
    texts.iter().map(PathBuf::from).collect()
}

#[test]
fn cap_021_the_fixed_folders_with_the_home_folder_come_first_then_path() {
    let found = candidates(
        Some(OsStr::new("/usr/bin:/bin")),
        Some(Path::new("/Users/op")),
        &TOOL_FOLDERS,
    );
    assert_eq!(
        found,
        paths(&[
            "/Users/op/.cargo/bin",
            "/opt/homebrew/opt/rustup/bin",
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/Users/op/.bun/bin",
            "/Users/op/.local/bin",
            "/Users/op/.docker/bin",
            "/Users/op/.nix-profile/bin",
            "/nix/var/nix/profiles/default/bin",
            "/usr/bin",
            "/bin",
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
    let homebrew_first = candidates(
        Some(OsStr::new("/usr/bin:/opt/homebrew/bin")),
        None,
        &["/opt/homebrew/bin"],
    );
    assert_eq!(homebrew_first, paths(&["/opt/homebrew/bin", "/usr/bin"]));
    let relative_home = candidates(None, Some(Path::new("home")), &["~/.cargo/bin"]);
    assert_eq!(relative_home, Vec::<PathBuf>::new());
}

#[test]
fn cap_021_apple_s_placeholders_run_on_macos_only_with_the_developer_tools() {
    for shim in ["/usr/bin/git", "/usr/bin/xcodebuild"] {
        let shim = Path::new(shim);
        assert!(!may_run(shim, Os::Macos, false), "{shim:?}");
        assert!(may_run(shim, Os::Macos, true), "{shim:?}");
        assert!(
            may_run(shim, Os::Linux, false),
            "{shim:?} is git itself on Linux"
        );
    }
    for other in [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/cargo",
        "/usr/bin/gitk",
        "/usr/bin/sub/git",
    ] {
        assert!(may_run(Path::new(other), Os::Macos, false), "{other}");
    }
}
