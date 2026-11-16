//! The Mach-O checks on real files (UPD-011, S12 2.3, D-4): the walk that finds every Mach-O
//! file of the app, and on macOS the planned `codesign` commands against files signed ad hoc in
//! a temporary folder.

use std::{fs, os::unix::fs::symlink};

use crate::builder::{artifact, carried::on_disk, error::BuilderError};

#[test]
fn s12_s_8_an_app_or_programs_folder_reached_through_a_link_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(root.path()).unwrap();
    let clone = base.join("clone");
    let real = base.join("elsewhere/Anima.ing.app");
    fs::create_dir_all(real.join("Contents/MacOS")).unwrap();
    fs::create_dir_all(&clone).unwrap();
    let app = clone.join("Anima.ing.app");
    symlink(&real, &app).unwrap();
    assert_eq!(
        artifact::mac_executable(&clone, &app),
        Err(BuilderError::Bundle)
    );
    assert_eq!(
        on_disk::mach_o_files(&clone, &app),
        Err(BuilderError::Bundle)
    );
    fs::remove_file(&app).unwrap();
    fs::create_dir_all(app.join("Contents")).unwrap();
    symlink(real.join("Contents/MacOS"), app.join("Contents/MacOS")).unwrap();
    assert_eq!(
        artifact::mac_executable(&clone, &app),
        Err(BuilderError::Bundle)
    );
    assert_eq!(artifact::own_folder(&clone, &app), Ok(app.clone()));
}

#[test]
fn s12_d_4_every_mach_o_file_below_the_app_is_found_and_no_link_is_followed() {
    let root = tempfile::tempdir().unwrap();
    let clone = fs::canonicalize(root.path()).unwrap().join("clone");
    let app = clone.join("Anima.ing.app");
    let lib = app.join("Contents/Resources/database/lib/postgresql");
    fs::create_dir_all(&lib).unwrap();
    fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    let fat = [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 2];
    fs::write(app.join("Contents/MacOS/anima"), fat).unwrap();
    fs::write(lib.join("amcheck.dylib"), [0xcf, 0xfa, 0xed, 0xfe]).unwrap();
    // Named for a text file, found by its bytes.
    fs::write(lib.join("README"), fat).unwrap();
    fs::write(app.join("Contents/Info.plist"), b"<?xml").unwrap();
    fs::write(lib.join("short"), [0xca, 0xfe]).unwrap();
    let outside = root.path().join("outside");
    fs::write(&outside, fat).unwrap();
    symlink(&outside, lib.join("linked.dylib")).unwrap();
    symlink(root.path(), app.join("Contents/escape")).unwrap();
    assert_eq!(
        on_disk::mach_o_files(&clone, &app).unwrap(),
        [
            app.join("Contents/MacOS/anima"),
            lib.join("README"),
            lib.join("amcheck.dylib"),
        ]
    );
    // A name with a line break could make a line of codesign's output look like another.
    fs::write(lib.join("x\nTimestamp=now"), b"text").unwrap();
    assert_eq!(
        on_disk::mach_o_files(&clone, &app),
        Err(BuilderError::Bundle)
    );
}

/// The planned commands against real files signed ad hoc in a temporary folder.
#[cfg(target_os = "macos")]
mod signed_here {
    use std::{path::Path, process::Command, sync::Arc};

    use crate::builder::{
        carried::{mach_o, signer::Signer},
        child::Running,
        environment::Environment,
        error::BuilderError,
        plan,
    };

    const IDENTITY: &str = "Developer ID Application: Stephen Okita (7GDLG48PV6)";
    const ENTITLED: &str = r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>com.apple.security.cs.disable-library-validation</key><true/></dict></plist>"#;

    /// Runs `command` as `/usr/bin/<program>` in a scrubbed environment.
    async fn run(command: &plan::Command) -> Result<Vec<String>, BuilderError> {
        let running = Running::default();
        let claim = running.claim(Arc::new(|_| {})).unwrap();
        let env = Environment::from_parent(std::env::vars_os());
        let program = Path::new("/usr/bin").join(command.program.name());
        claim.read(command, &program, env.for_step(&[])).await
    }

    fn signed_copy(folder: &Path, name: &str, extra: &[&str]) -> std::path::PathBuf {
        let file = folder.join(name);
        std::fs::copy("/usr/bin/true", &file).unwrap();
        let status = Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .args(extra)
            .arg(&file)
            .output()
            .unwrap()
            .status;
        assert!(status.success());
        file
    }

    #[tokio::test]
    async fn upd_011_a_file_signed_ad_hoc_fails_each_check_it_can_fail() {
        let root = tempfile::tempdir().unwrap();
        let entitlements = root.path().join("entitlements.plist");
        std::fs::write(&entitlements, ENTITLED).unwrap();
        let hardened = signed_copy(root.path(), "hardened", &["--options", "runtime"]);
        let entitled = signed_copy(
            root.path(),
            "entitled",
            &[
                "--options",
                "runtime",
                "--entitlements",
                entitlements.to_str().unwrap(),
            ],
        );
        let team = Signer::from_identity(IDENTITY).unwrap().team;
        for file in [&hardened, &entitled, Path::new("/usr/bin/true")] {
            assert_eq!(
                run(&mach_o::verify(file, &team)).await,
                Err(BuilderError::StepFailed),
                "{file:?}"
            );
        }
        let display = run(&mach_o::display(&hardened, "arm64")).await.unwrap();
        assert!(display.iter().any(|line| line.contains("(adhoc,runtime)")));
        assert!(!mach_o::hardened_and_timestamped(&display));
        let none = run(&mach_o::entitlements(&hardened, "x86_64"))
            .await
            .unwrap();
        assert!(mach_o::no_entitlements(&none));
        let some = run(&mach_o::entitlements(&entitled, "arm64"))
            .await
            .unwrap();
        assert!(!mach_o::no_entitlements(&some));
    }
}
