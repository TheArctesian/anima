//! The six programs in the release (INS-002, INS-003, OD-16, OD-17, S12 2.1 and 2.2): their
//! build, where each is put for the bundle, the set the signed app may hold, and the Rust targets
//! a universal build needs before a job is claimed (BLD-028).

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

use super::args;
use crate::builder::{
    artifact, carried,
    carried::{link, rust_programs},
    environment::Environment,
    error::BuilderError,
    job::{Target, Version},
    plan::Program,
    tools::{self, Tool},
};

const STAGE: &str = "/c/apps/desktop/src-tauri/binaries";

#[test]
fn ins_003_the_five_rust_programs_are_built_in_one_cargo_run_with_the_release_version() {
    let version = Version::parse("0.1.12").unwrap();
    let linux = rust_programs::build(Path::new("/c"), Target::LinuxX86_64, &version);
    assert_eq!(linux.program, Program::Cargo);
    assert_eq!(linux.cwd, Path::new("/c"));
    assert_eq!(
        args(&linux),
        [
            "build",
            "--release",
            "--locked",
            "--package",
            "anima-cli",
            "--bin",
            "anima",
            "--package",
            "anima-local-hub",
            "--bin",
            "anima-local-hub",
            "--package",
            "anima-hub",
            "--bin",
            "anima-hub",
            "--package",
            "anima-database",
            "--bin",
            "anima-database",
            "--package",
            "anima-gateway",
            "--bin",
            "anima-gateway",
        ]
    );
    assert_eq!(linux.env, [("ANIMA_VERSION", "0.1.12".to_owned())]);
    let mac = rust_programs::build(Path::new("/c"), Target::DarwinUniversal, &version);
    assert_eq!(args(&mac)[..23], args(&linux)[..]);
    assert_eq!(
        args(&mac)[23..],
        [
            "--target",
            "aarch64-apple-darwin",
            "--target",
            "x86_64-apple-darwin"
        ]
    );
    assert_eq!(mac.env, linux.env);
}

#[test]
fn ins_002_each_half_is_put_under_the_name_tauri_reads_for_its_target() {
    let clone = Path::new("/c");
    let linux = rust_programs::copies(clone, Target::LinuxX86_64);
    assert_eq!(linux.len(), 5);
    assert_eq!(
        linux[0],
        (
            clone.join("target/release/anima"),
            Path::new(STAGE).join("anima-x86_64-unknown-linux-gnu")
        )
    );
    assert_eq!(
        carried::staged(clone, Target::LinuxX86_64, "anima-hub"),
        Path::new(STAGE).join("anima-hub-x86_64-unknown-linux-gnu")
    );
    assert!(carried::joined(clone, Target::LinuxX86_64, "anima").is_none());
    let halves = rust_programs::copies(clone, Target::DarwinUniversal);
    assert_eq!(halves.len(), 10);
    assert_eq!(
        halves[3],
        (
            clone.join("target/x86_64-apple-darwin/release/anima-local-hub"),
            Path::new(STAGE).join("anima-local-hub-x86_64-apple-darwin")
        )
    );
    let lipo = carried::joined(clone, Target::DarwinUniversal, "anima-gateway").unwrap();
    assert_eq!(lipo.program, Program::Lipo);
    assert_eq!(
        args(&lipo),
        [
            "-create",
            "-output",
            "/c/apps/desktop/src-tauri/binaries/anima-gateway-universal-apple-darwin",
            "/c/apps/desktop/src-tauri/binaries/anima-gateway-aarch64-apple-darwin",
            "/c/apps/desktop/src-tauri/binaries/anima-gateway-x86_64-apple-darwin"
        ]
    );
    assert_eq!(
        carried::staged(clone, Target::DarwinUniversal, "anima"),
        Path::new(STAGE).join("anima-universal-apple-darwin")
    );
}

#[test]
fn s12_d_2_anima_link_is_built_by_bin_sh_from_the_clone_and_staged_per_platform() {
    let clone = Path::new("/c");
    let build = link::build(clone);
    assert_eq!(build.program, Program::Sh);
    assert_eq!(Program::Sh.fixed_path(), Some(Path::new("/bin/sh")));
    assert_eq!(
        (args(&build), build.cwd.as_path()),
        (vec!["link/build.sh"], clone)
    );
    assert_eq!(build.env, Vec::new());
    assert_eq!(
        link::copies(clone, Target::LinuxX86_64),
        [(
            clone.join("link/bin/anima-link-linux-amd64"),
            Path::new(STAGE).join("anima-link-x86_64-unknown-linux-gnu")
        )]
    );
    assert_eq!(
        link::copies(clone, Target::DarwinUniversal),
        [
            (
                clone.join("link/bin/anima-link-darwin-arm64"),
                Path::new(STAGE).join("anima-link-aarch64-apple-darwin")
            ),
            (
                clone.join("link/bin/anima-link-darwin-amd64"),
                Path::new(STAGE).join("anima-link-x86_64-apple-darwin")
            )
        ]
    );
}

#[test]
fn ins_002_the_bundle_names_the_six_programs_and_the_tree() {
    let bins: Vec<&str> = rust_programs::PROGRAMS
        .iter()
        .map(|(_, binary)| *binary)
        .chain([link::PROGRAM])
        .collect();
    assert_eq!(bins, carried::PROGRAMS);
    assert_eq!(
        carried::bundle_settings().to_string(),
        r#"{"externalBin":["binaries/anima","binaries/anima-local-hub","binaries/anima-hub","binaries/anima-database","binaries/anima-gateway","binaries/anima-link"],"resources":{"database":"database"}}"#
    );
}

#[test]
fn ins_002_only_a_64_bit_x86_64_elf_file_is_a_linux_program() {
    let mut elf = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
    elf.extend([0; 10]);
    elf.extend([62, 0]);
    assert!(carried::is_x86_64_elf(&elf));
    let mut arm = elf.clone();
    arm[18] = 183;
    assert!(!carried::is_x86_64_elf(&arm));
    let mut thirty_two = elf.clone();
    thirty_two[4] = 1;
    assert!(!carried::is_x86_64_elf(&thirty_two));
    assert!(!carried::is_x86_64_elf(&elf[..18]));
    assert!(!carried::is_x86_64_elf(
        b"-----BEGIN OPENSSH PRIVATE KEY-----"
    ));
}

#[test]
fn s12_2_3_the_mac_app_holds_exactly_its_own_executable_and_the_six_programs() {
    let root = tempfile::tempdir().unwrap();
    let clone = fs::canonicalize(root.path()).unwrap();
    let app = clone.join("Anima.app");
    let macos = app.join("Contents/MacOS");
    fs::create_dir_all(&macos).unwrap();
    fs::write(macos.join("anima-desktop"), b"app").unwrap();
    for program in &carried::PROGRAMS[..5] {
        fs::write(macos.join(program), b"program").unwrap();
    }
    assert_eq!(
        artifact::mac_executable(&clone, &app),
        Err(BuilderError::Bundle),
        "anima-link is missing"
    );
    fs::write(macos.join("anima-link"), b"link").unwrap();
    assert_eq!(
        artifact::mac_executable(&clone, &app),
        Ok(macos.join("anima-desktop"))
    );
    fs::write(macos.join("anima-agent"), b"other").unwrap();
    assert_eq!(
        artifact::mac_executable(&clone, &app),
        Err(BuilderError::Bundle)
    );
}

#[test]
fn ins_002_staging_copies_cargos_output_and_never_writes_through_a_planted_link() {
    let root = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(root.path()).unwrap();
    let clone = base.join("clone");
    let built = clone.join("target/release/anima");
    fs::create_dir_all(built.parent().unwrap()).unwrap();
    fs::write(&built, b"\x7fELF built").unwrap();
    // Cargo's second name for its output.
    fs::create_dir_all(clone.join("target/release/deps")).unwrap();
    fs::hard_link(&built, clone.join("target/release/deps/anima-1234")).unwrap();
    let outside = base.join("bashrc");
    fs::write(&outside, b"keep me").unwrap();
    let staged = clone.join("apps/desktop/src-tauri/binaries/anima-x86_64-unknown-linux-gnu");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    symlink(&outside, &staged).unwrap();
    artifact::stage(&clone, &built, &staged).unwrap();
    assert_eq!(fs::read(&outside).unwrap(), b"keep me");
    assert_eq!(fs::read(&staged).unwrap(), b"\x7fELF built");
    let mode = fs::symlink_metadata(&staged).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    assert_eq!(
        artifact::header(&clone, &staged, 4).unwrap(),
        b"\x7fELF".to_vec()
    );
    let escape = clone.join("target/escape");
    symlink(&outside, &escape).unwrap();
    assert_eq!(
        artifact::stage(&clone, &escape, &staged),
        Err(BuilderError::Bundle)
    );
}

/// `exists` for a sysroot `/toolchain` that has the standard library of each of `triples`.
fn toolchain_with(triples: &'static [&'static str]) -> impl Fn(&Path) -> bool {
    move |path: &Path| {
        triples
            .iter()
            .any(|triple| path == Path::new("/toolchain/lib/rustlib").join(triple).join("lib"))
    }
}

#[test]
fn bld_028_a_universal_build_needs_both_darwin_standard_libraries() {
    let sysroot = Some(Path::new("/toolchain"));
    let both = toolchain_with(&["aarch64-apple-darwin", "x86_64-apple-darwin"]);
    assert_eq!(
        tools::rust_targets(Target::DarwinUniversal, sysroot, both),
        []
    );
    // A system toolchain, as Homebrew's, carries the host's library only.
    let host = toolchain_with(&["aarch64-apple-darwin"]);
    assert_eq!(
        tools::rust_targets(Target::DarwinUniversal, sysroot, host),
        [Tool::RustTargetX86_64Darwin]
    );
    assert_eq!(
        tools::rust_targets(Target::DarwinUniversal, None, |_| true),
        [Tool::RustTargetAarch64Darwin, Tool::RustTargetX86_64Darwin]
    );
    assert_eq!(
        tools::rust_targets(Target::LinuxX86_64, None, |_| false),
        []
    );
    let json = serde_json::to_string(&[Tool::RustTargetX86_64Darwin]).unwrap();
    assert_eq!(json, r#"["rust-target-x86_64-apple-darwin"]"#);
}

#[test]
fn bld_028_the_sysroot_is_read_from_the_rustc_the_steps_find() {
    let env = Environment::from_parent(std::env::vars_os());
    let sysroot = tools::readings::rust_sysroot(&env, Path::new("/")).expect("a rustc");
    assert!(sysroot.join("lib/rustlib").is_dir(), "{sysroot:?}");
}
