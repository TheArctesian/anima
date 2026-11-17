//! The five Rust programs a release carries (INS-002, INS-003, OD-17, S12 2.2 step 2): one
//! `cargo build` from the clone with the release version in `ANIMA_VERSION`, for the targets the
//! app is built for. Pure.

use std::path::{Path, PathBuf};

use super::{staged_as, triples};
use crate::builder::{
    artifact,
    job::{Target, Version},
    plan::{Command, Program, command},
};

/// Each program's cargo package and binary, in the order of [`super::programs`].
pub const PROGRAMS: [(&str, &str); 5] = [
    ("anima-cli", "anima"),
    ("anima-local-hub", "anima-local-hub"),
    ("anima-hub", "anima-hub"),
    ("anima-database", "anima-database"),
    ("anima-gateway", "anima-gateway"),
];
/// The variable `crates/anima-cli/build.rs` reads the release version from.
const VERSION_VARIABLE: &str = "ANIMA_VERSION";

/// Where cargo leaves `binary` for `triple`. Linux builds for the host, as `tauri build` does
/// there, so its output has no target folder.
fn built(clone: &Path, target: Target, triple: &str, binary: &str) -> PathBuf {
    match target {
        Target::LinuxX86_64 => artifact::target_folder(clone).join(format!("release/{binary}")),
        Target::DarwinUniversal => {
            artifact::target_folder(clone).join(format!("{triple}/release/{binary}"))
        }
    }
}

/// One `cargo build` of the five programs for every triple of `target`, with the release version
/// (OD-17).
pub fn build(clone: &Path, target: Target, version: &Version) -> Command {
    let mut args = vec!["build", "--release", "--locked"];
    for (package, binary) in PROGRAMS {
        args.extend(["--package", package, "--bin", binary]);
    }
    if target == Target::DarwinUniversal {
        for triple in triples(target) {
            args.extend(["--target", triple]);
        }
    }
    let mut cargo = command(Program::Cargo, clone, &args);
    cargo
        .env
        .push((VERSION_VARIABLE, version.as_str().to_owned()));
    cargo
}

/// Each built program for each triple, and the name Tauri reads it under.
pub fn copies(clone: &Path, target: Target) -> Vec<(PathBuf, PathBuf)> {
    PROGRAMS
        .iter()
        .flat_map(|(_, binary)| {
            triples(target).iter().map(move |triple| {
                (
                    built(clone, target, triple, binary),
                    staged_as(clone, binary, triple),
                )
            })
        })
        .collect()
}
