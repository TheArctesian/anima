//! What a release carries for a hub on this computer (INS-002, OD-16, S12 packaging Section 2.1):
//! six programs beside the app's own executable and the PostgreSQL tree in its resources, each
//! built from the job's commit in the clone. Tauri needs `binaries/<program>-<triple>` for each
//! Rust target it compiles, and for the universal bundle `binaries/<program>-universal-apple-darwin`,
//! which `lipo` makes from the two halves, as Tauri makes the app's own universal binary. A missing
//! file fails `tauri build`. Pure; the I/O is in `on_disk` and the steps in `steps`.

pub mod committed;
pub mod link;
pub mod mach_o;
pub mod on_disk;
pub mod rust_programs;
mod steps;
pub mod tree;

use std::path::{Path, PathBuf};

use super::{
    job::Target,
    plan::{Command, Program, command},
};

/// The programs beside the app's own executable, as `bundle.externalBin` names them under
/// `binaries/`: the five Rust programs, then `anima-link`.
pub const PROGRAMS: [&str; 6] = [
    "anima",
    "anima-local-hub",
    "anima-hub",
    "anima-database",
    "anima-gateway",
    "anima-link",
];
/// The folder Tauri reads the programs from, in the clone.
const STAGE: &str = "apps/desktop/src-tauri/binaries";
const UNIVERSAL: &str = "universal-apple-darwin";
const LINUX_TRIPLE: &str = "x86_64-unknown-linux-gnu";
const MAC_TRIPLES: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// The `bundle` settings that make Tauri carry the six programs and the tree, whose folder
/// `apps/desktop/src-tauri/database` becomes `database` in the resources.
pub fn bundle_settings() -> serde_json::Value {
    let programs: Vec<String> = PROGRAMS
        .iter()
        .map(|program| format!("binaries/{program}"))
        .collect();
    serde_json::json!({ "externalBin": programs, "resources": { "database": "database" } })
}

/// The Rust targets `tauri build` compiles for `target`.
fn triples(target: Target) -> &'static [&'static str] {
    match target {
        Target::LinuxX86_64 => &[LINUX_TRIPLE],
        Target::DarwinUniversal => &MAC_TRIPLES,
    }
}

fn staged_as(clone: &Path, program: &str, triple: &str) -> PathBuf {
    clone.join(STAGE).join(format!("{program}-{triple}"))
}

/// The file of `program` the bundle takes for `target`.
pub fn staged(clone: &Path, target: Target, program: &str) -> PathBuf {
    match target {
        Target::LinuxX86_64 => staged_as(clone, program, LINUX_TRIPLE),
        Target::DarwinUniversal => staged_as(clone, program, UNIVERSAL),
    }
}

/// `lipo` of the two macOS halves of `program` into the universal file the bundle takes; none on
/// Linux.
pub fn joined(clone: &Path, target: Target, program: &str) -> Option<Command> {
    (target == Target::DarwinUniversal).then(|| {
        let mut lipo = command(Program::Lipo, clone, &["-create", "-output"]);
        lipo.args
            .push(staged(clone, target, program).into_os_string());
        for triple in MAC_TRIPLES {
            lipo.args
                .push(staged_as(clone, program, triple).into_os_string());
        }
        lipo
    })
}

/// Whether `header`, the first bytes of a file, starts a 64-bit little-endian x86-64 ELF
/// program: `e_type` an executable (2) or a position-independent one (3), `e_machine` 62.
pub fn is_x86_64_elf(header: &[u8]) -> bool {
    matches!(header, [0x7f, b'E', b'L', b'F', 2, 1, ..])
        && matches!(header.get(16..18), Some([2 | 3, 0]))
        && header.get(18..20) == Some(&[62, 0][..])
}
