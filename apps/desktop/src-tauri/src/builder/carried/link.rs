//! `anima-link`, the Go program a release carries (S12 2.2): `link/build.sh` of the clone, run by
//! `/bin/sh` (D-2), writes one program for each platform to `link/bin/`; the ones the app needs
//! are put under the names Tauri reads. Go runs with a fixed environment: no Go settings file of
//! the builder's user, no other toolchain, no workspace, no change of `go.mod`, and modules only
//! from the public proxy, each checked against the checksum database. Pure.

use std::path::{Path, PathBuf};

use super::{staged_as, triples};
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
/// The Go module, relative to the clone.
const MODULE: &str = "link";
/// The files of the clone the recipe runs or reads, each checked against the commit before.
pub const COMMITTED: [&str; 3] = [RECIPE, "link/go.mod", "link/go.sum"];
/// Files git ignores that would change what Go builds; none may be in the clone.
pub const ABSENT: [&str; 2] = ["link/go.work", "link/vendor"];
/// The Go environment of both commands.
const GO_ENVIRONMENT: [(&str, &str); 11] = [
    ("GOENV", "off"),
    ("GOTOOLCHAIN", "local"),
    ("GOWORK", "off"),
    ("GOFLAGS", "-mod=readonly"),
    ("GOPROXY", "https://proxy.golang.org"),
    ("GOSUMDB", "sum.golang.org"),
    ("GONOSUMDB", ""),
    ("GONOSUMCHECK", ""),
    ("GONOPROXY", ""),
    ("GOINSECURE", ""),
    ("GOPRIVATE", ""),
];

fn with_go_environment(mut step: Command) -> Command {
    step.env.extend(
        GO_ENVIRONMENT
            .iter()
            .map(|(name, value)| (*name, (*value).to_owned())),
    );
    step
}

/// `go mod verify` in `link/`: the modules in the cache are the ones `go.sum` names.
pub fn verify(clone: &Path) -> Command {
    with_go_environment(command(
        Program::Go,
        &clone.join(MODULE),
        &["mod", "verify"],
    ))
}

/// The Go platform (`<os>-<arch>`) of a Rust triple the app is built for.
fn go_platform(triple: &str) -> String {
    let os = if triple.ends_with("-apple-darwin") {
        "darwin"
    } else if triple.ends_with("-windows-msvc") {
        "windows"
    } else {
        "linux"
    };
    let arch = if triple.starts_with("aarch64-") {
        "arm64"
    } else {
        "amd64"
    };
    format!("{os}-{arch}")
}

/// `/bin/sh link/build.sh`, from the clone. Go is found on the step's `PATH`.
pub fn build(clone: &Path) -> Command {
    with_go_environment(command(Program::Sh, clone, &[RECIPE]))
}

/// Each program the recipe wrote for `target`, and the name Tauri reads it under.
pub fn copies(clone: &Path, target: Target) -> Vec<(PathBuf, PathBuf)> {
    triples(target)
        .iter()
        .map(|triple| {
            (
                clone.join(OUTPUT).join(format!(
                    "{PROGRAM}-{}{}",
                    go_platform(triple),
                    if triple.ends_with("-windows-msvc") {
                        ".exe"
                    } else {
                        ""
                    }
                )),
                staged_as(clone, PROGRAM, triple),
            )
        })
        .collect()
}

/// Native Windows recipe and module definitions checked before executing Go.
pub const WINDOWS_COMMITTED: [&str; 3] = ["link/build-windows.ts", "link/go.mod", "link/go.sum"];

pub fn build_for(clone: &Path, target: Target) -> Command {
    if target == Target::WindowsX86_64 {
        with_go_environment(command(Program::Bun, clone, &["link/build-windows.ts"]))
    } else {
        build(clone)
    }
}
