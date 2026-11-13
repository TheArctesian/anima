//! The `anima` program a release carries (INS-002, INS-003, OD-16, OD-17): built from the job's
//! commit with the release version in `ANIMA_VERSION`, for the targets the app is built for, and
//! put where `bundle.externalBin` finds it. Tauri needs `binaries/anima-<triple>` for each Rust
//! target it compiles, and for the universal bundle `binaries/anima-universal-apple-darwin`, which
//! `lipo` makes from the two halves, as Tauri makes the app's own universal binary. A missing file
//! fails `tauri build`. Pure.

use std::path::{Path, PathBuf};

use super::{
    job::{Target, Version},
    plan::{Command, Program, command},
};

/// The `bundle.externalBin` entry, relative to `apps/desktop/src-tauri`.
pub const EXTERNAL_BIN: &str = "binaries/anima";
/// The folder Tauri reads `EXTERNAL_BIN` from, in the clone.
const STAGE: &str = "apps/desktop/src-tauri/binaries";
/// The variable `crates/anima-cli/build.rs` reads the release version from.
const VERSION_VARIABLE: &str = "ANIMA_VERSION";
const UNIVERSAL: &str = "universal-apple-darwin";

/// The Rust targets `tauri build` compiles for `target`.
fn triples(target: Target) -> &'static [&'static str] {
    match target {
        Target::LinuxX86_64 => &["x86_64-unknown-linux-gnu"],
        Target::DarwinUniversal => &["aarch64-apple-darwin", "x86_64-apple-darwin"],
    }
}

/// Where cargo leaves `anima` for `triple`. Linux builds for the host, as `tauri build` does there,
/// so its output has no target folder.
fn built(clone: &Path, target: Target, triple: &str) -> PathBuf {
    match target {
        Target::LinuxX86_64 => clone.join("target/release/anima"),
        Target::DarwinUniversal => clone.join(format!("target/{triple}/release/anima")),
    }
}

fn staged_as(clone: &Path, triple: &str) -> PathBuf {
    clone.join(STAGE).join(format!("anima-{triple}"))
}

/// `cargo build` of `anima` for every triple of `target`, with the release version (OD-17).
pub fn build(clone: &Path, target: Target, version: &Version) -> Command {
    let mut args = vec![
        "build",
        "--release",
        "--locked",
        "--package",
        "anima-cli",
        "--bin",
        "anima",
    ];
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

/// Each built `anima` and the name Tauri reads it under.
pub fn copies(clone: &Path, target: Target) -> Vec<(PathBuf, PathBuf)> {
    triples(target)
        .iter()
        .map(|triple| (built(clone, target, triple), staged_as(clone, triple)))
        .collect()
}

/// `lipo` of the two macOS halves into the universal file the bundle takes; none on Linux.
pub fn universal(clone: &Path, target: Target) -> Option<Command> {
    (target == Target::DarwinUniversal).then(|| {
        let mut lipo = command(Program::Lipo, clone, &["-create", "-output"]);
        lipo.args.push(staged_as(clone, UNIVERSAL).into_os_string());
        for (_, half) in copies(clone, target) {
            lipo.args.push(half.into_os_string());
        }
        lipo
    })
}

/// The file the bundle step puts in the app for `target`.
pub fn staged(clone: &Path, target: Target) -> PathBuf {
    match target {
        Target::LinuxX86_64 => staged_as(clone, triples(target)[0]),
        Target::DarwinUniversal => staged_as(clone, UNIVERSAL),
    }
}

/// `lipo -archs` of a macOS `anima`; its output must name `x86_64` and `arm64`.
pub fn archs(program: &Path) -> Command {
    let folder = program.parent().unwrap_or(program);
    let mut lipo = command(Program::Lipo, folder, &["-archs"]);
    lipo.args.push(program.as_os_str().to_owned());
    lipo
}

/// `codesign --verify --strict` of the `anima` in the signed app (OD-16).
pub fn signature(program: &Path) -> Command {
    let folder = program.parent().unwrap_or(program);
    let mut codesign = command(Program::Codesign, folder, &["--verify", "--strict"]);
    codesign.args.push(program.as_os_str().to_owned());
    codesign
}

/// Whether `header`, the first bytes of a file, starts a 64-bit little-endian x86-64 ELF
/// executable: the Linux `anima` (`e_machine` 62).
pub fn is_x86_64_elf(header: &[u8]) -> bool {
    matches!(header, [0x7f, b'E', b'L', b'F', 2, 1, ..] if header.get(18..20) == Some(&[62, 0][..]))
}
