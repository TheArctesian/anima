//! The `anima` program in the release (INS-002, INS-003, OD-16, OD-17): its build, where it is
//! put for the bundle, and the checks of the bundle that carries it.

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

use crate::builder::{
    artifact,
    error::BuilderError,
    job::{Target, Version},
    plan::Program,
    sidecar,
};

fn args(command: &crate::builder::plan::Command) -> Vec<&str> {
    command.args.iter().map(|a| a.to_str().unwrap()).collect()
}

#[test]
fn ins_003_anima_is_built_from_the_clone_with_the_release_version() {
    let version = Version::parse("0.1.12").unwrap();
    let linux = sidecar::build(Path::new("/c"), Target::LinuxX86_64, &version);
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
            "anima"
        ]
    );
    assert_eq!(linux.env, [("ANIMA_VERSION", "0.1.12".to_owned())]);
    let mac = sidecar::build(Path::new("/c"), Target::DarwinUniversal, &version);
    assert_eq!(
        args(&mac)[7..],
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
    let stage = "/c/apps/desktop/src-tauri/binaries";
    assert_eq!(
        sidecar::copies(clone, Target::LinuxX86_64),
        [(
            clone.join("target/release/anima"),
            Path::new(stage).join("anima-x86_64-unknown-linux-gnu")
        )]
    );
    assert_eq!(
        sidecar::staged(clone, Target::LinuxX86_64),
        Path::new(stage).join("anima-x86_64-unknown-linux-gnu")
    );
    assert!(sidecar::universal(clone, Target::LinuxX86_64).is_none());
    let halves = sidecar::copies(clone, Target::DarwinUniversal);
    assert_eq!(
        halves[1],
        (
            clone.join("target/x86_64-apple-darwin/release/anima"),
            Path::new(stage).join("anima-x86_64-apple-darwin")
        )
    );
    let lipo = sidecar::universal(clone, Target::DarwinUniversal).unwrap();
    assert_eq!(lipo.program, Program::Lipo);
    assert_eq!(
        args(&lipo),
        [
            "-create",
            "-output",
            "/c/apps/desktop/src-tauri/binaries/anima-universal-apple-darwin",
            "/c/apps/desktop/src-tauri/binaries/anima-aarch64-apple-darwin",
            "/c/apps/desktop/src-tauri/binaries/anima-x86_64-apple-darwin"
        ]
    );
    assert_eq!(
        sidecar::staged(clone, Target::DarwinUniversal),
        Path::new(stage).join("anima-universal-apple-darwin")
    );
}

#[test]
fn od_016_the_bundled_anima_is_checked_for_both_architectures_and_its_signature() {
    let program = Path::new("/c/Anima.app/Contents/MacOS/anima");
    let codesign = sidecar::signature(program);
    assert_eq!(codesign.program, Program::Codesign);
    assert_eq!(
        args(&codesign),
        ["--verify", "--strict", "/c/Anima.app/Contents/MacOS/anima"]
    );
    let lipo = sidecar::archs(program);
    assert_eq!(
        (lipo.program, args(&lipo)),
        (
            Program::Lipo,
            vec!["-archs", "/c/Anima.app/Contents/MacOS/anima"]
        )
    );
}

#[test]
fn ins_002_only_a_64_bit_x86_64_elf_file_is_the_linux_anima() {
    let mut elf = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
    elf.extend([0; 10]);
    elf.extend([62, 0]);
    assert!(sidecar::is_x86_64_elf(&elf));
    let mut arm = elf.clone();
    arm[18] = 183;
    assert!(!sidecar::is_x86_64_elf(&arm));
    let mut thirty_two = elf.clone();
    thirty_two[4] = 1;
    assert!(!sidecar::is_x86_64_elf(&thirty_two));
    assert!(!sidecar::is_x86_64_elf(&elf[..18]));
    assert!(!sidecar::is_x86_64_elf(
        b"-----BEGIN OPENSSH PRIVATE KEY-----"
    ));
}

#[test]
fn ins_002_the_mac_app_holds_exactly_its_own_executable_and_anima() {
    let root = tempfile::tempdir().unwrap();
    let clone = fs::canonicalize(root.path()).unwrap();
    let app = clone.join("Anima.app");
    let macos = app.join("Contents/MacOS");
    fs::create_dir_all(&macos).unwrap();
    fs::write(macos.join("anima-desktop"), b"app").unwrap();
    assert_eq!(
        artifact::mac_binaries(&clone, &app),
        Err(BuilderError::Bundle),
        "anima is missing"
    );
    fs::write(macos.join("anima"), b"cli").unwrap();
    let binaries = artifact::mac_binaries(&clone, &app).unwrap();
    assert_eq!(binaries.app, macos.join("anima-desktop"));
    assert_eq!(binaries.command_line, macos.join("anima"));
    fs::write(macos.join("anima-agent"), b"other").unwrap();
    assert_eq!(
        artifact::mac_binaries(&clone, &app),
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
