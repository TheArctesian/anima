//! `anima-link`, the Go program a release carries (S12 2.2 step 3): `link/build.sh` of the clone,
//! run by `/bin/sh` (D-2), writes one program for each platform to `link/bin/`; the ones the app
//! needs are put under the names Tauri reads. Pure.

use std::path::{Path, PathBuf};

use super::staged_as;
use crate::builder::{
    job::Target,
    plan::{Command, Program, command},
};

/// The program's name in the bundle.
pub const PROGRAM: &str = "anima-link";
/// The recipe, relative to the clone.
const RECIPE: &str = "link/build.sh";
/// Where the recipe writes `anima-link-<os>-<arch>`, relative to the clone.
const OUTPUT: &str = "link/bin";

/// The Go platform of each Rust triple the app is built for.
fn platforms(target: Target) -> &'static [(&'static str, &'static str)] {
    match target {
        Target::LinuxX86_64 => &[("linux-amd64", "x86_64-unknown-linux-gnu")],
        Target::DarwinUniversal => &[
            ("darwin-arm64", "aarch64-apple-darwin"),
            ("darwin-amd64", "x86_64-apple-darwin"),
        ],
    }
}

/// `/bin/sh link/build.sh`, from the clone. Go is found on the step's `PATH`.
pub fn build(clone: &Path) -> Command {
    command(Program::Sh, clone, &[RECIPE])
}

/// Each program the recipe wrote for `target`, and the name Tauri reads it under.
pub fn copies(clone: &Path, target: Target) -> Vec<(PathBuf, PathBuf)> {
    platforms(target)
        .iter()
        .map(|(platform, triple)| {
            (
                clone.join(OUTPUT).join(format!("{PROGRAM}-{platform}")),
                staged_as(clone, PROGRAM, triple),
            )
        })
        .collect()
}
