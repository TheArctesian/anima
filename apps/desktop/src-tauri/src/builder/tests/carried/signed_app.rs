//! The checks of the signed app run on real files (UPD-011, S12 D-3, D-4): a universal program
//! this computer compiles and signs ad hoc in a temporary folder, never with an identity. Each
//! copy fails a different check; a file that is not Mach-O is not checked.

use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use crate::builder::{
    carried::{mach_o::TeamId, programs, signer::Signer},
    child::Running,
    environment::Environment,
    error::BuilderError,
    plan::Repository,
    steps::Workshop,
};

const ENTITLEMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>com.apple.security.get-task-allow</key><true/></dict></plist>
"#;

/// A canonical temporary folder holding `clone/` and the compiled programs in `made/`.
struct Bench {
    _root: tempfile::TempDir,
    clone: PathBuf,
    made: PathBuf,
}

fn tool(program: &str, args: &[&str], cwd: &Path) {
    let status = process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "{program} {args:?}");
}

/// Compiles a universal program and signs copies of it ad hoc: `universal` with the hardened
/// runtime, `entitled` with an entitlement too, `thin` with the `arm64` slice only.
fn bench() -> Bench {
    let root = tempfile::tempdir().unwrap();
    let top = fs::canonicalize(root.path()).unwrap();
    let made = top.join("made");
    let clone = top.join("clone");
    fs::create_dir_all(&made).unwrap();
    fs::create_dir_all(&clone).unwrap();
    fs::write(made.join("main.c"), "int main(void) { return 0; }\n").unwrap();
    fs::write(made.join("entitlements.plist"), ENTITLEMENT).unwrap();
    tool(
        "/usr/bin/cc",
        &[
            "-arch",
            "x86_64",
            "-arch",
            "arm64",
            "-o",
            "universal",
            "main.c",
        ],
        &made,
    );
    tool(
        "/usr/bin/lipo",
        &["universal", "-thin", "arm64", "-output", "thin"],
        &made,
    );
    fs::copy(made.join("universal"), made.join("entitled")).unwrap();
    let sign = ["-s", "-", "-f", "--options", "runtime"];
    for file in ["universal", "thin"] {
        tool("/usr/bin/codesign", &[&sign[..], &[file]].concat(), &made);
    }
    let entitled = [
        &sign[..],
        &["--entitlements", "entitlements.plist", "entitled"],
    ]
    .concat();
    tool("/usr/bin/codesign", &entitled, &made);
    Bench {
        _root: root,
        clone,
        made,
    }
}

/// An app in the clone whose `Contents/MacOS` holds `files`, each a copy of a made program or,
/// when not made, a shell script.
fn app(bench: &Bench, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let app = bench.clone.join(name);
    let folder = app.join("Contents/MacOS");
    fs::create_dir_all(&folder).unwrap();
    for (file, made) in files {
        let from = bench.made.join(made);
        if from.exists() {
            fs::copy(from, folder.join(file)).unwrap();
        } else {
            fs::write(folder.join(file), "#!/bin/sh\n").unwrap();
        }
    }
    app
}

fn workshop<'a>(bench: &Bench, running: &'a Running) -> Workshop<'a> {
    Workshop {
        claim: running.claim(Arc::new(|_| {})).unwrap(),
        env: Environment::from_parent(std::env::vars_os()),
        repository: Repository::local(&bench.made),
        clone: bench.clone.clone(),
    }
}

fn team() -> TeamId {
    Signer::from_identity("Developer ID Application: Stephen Okita (7GDLG48PV6)")
        .unwrap()
        .team
}

#[tokio::test]
async fn upd_011_the_readings_of_an_ad_hoc_program_hold_both_architectures_and_fail() {
    let bench = bench();
    let running = Running::default();
    let workshop = workshop(&bench, &running);
    let file = bench.made.join("universal");
    let readings = workshop.mach_o_readings(&file, &team()).await.unwrap();
    let mut archs: Vec<&str> = readings
        .archs
        .iter()
        .flat_map(|l| l.split_whitespace())
        .collect();
    archs.sort_unstable();
    assert_eq!(archs, ["arm64", "x86_64"]);
    assert!(
        !readings.verified,
        "an ad hoc signature is not a Developer ID"
    );
    let read: Vec<&str> = readings.displays.iter().map(|(arch, _)| *arch).collect();
    assert_eq!(read, ["x86_64", "arm64"]);
    for (arch, lines) in &readings.displays {
        let directories = lines
            .iter()
            .filter(|l| l.starts_with("CodeDirectory "))
            .count();
        assert_eq!(directories, 1, "{arch}");
        assert!(lines.iter().any(|l| l.contains("adhoc,runtime")), "{arch}");
    }
    assert_eq!(readings.entitlements.len(), 2);
    let entitled = bench.made.join("entitled");
    let readings = workshop.mach_o_readings(&entitled, &team()).await.unwrap();
    for (arch, lines) in &readings.entitlements {
        assert!(lines.concat().contains("<key>"), "{arch}");
    }
}

#[tokio::test]
async fn upd_011_each_bad_mach_o_file_fails_the_app_and_other_files_are_not_checked() {
    let bench = bench();
    let running = Running::default();
    let workshop = workshop(&bench, &running);
    for made in ["universal", "entitled", "thin"] {
        let app = app(
            &bench,
            &format!("{made}.app"),
            &[("text", "script"), ("program", made)],
        );
        assert_eq!(
            workshop.signed_mach_o(&app, &team()).await,
            Err(BuilderError::Bundle),
            "{made}"
        );
    }
    let plain = app(
        &bench,
        "plain.app",
        &[("script", "script"), ("other", "script")],
    );
    fs::write(plain.join("Contents/Info.plist"), "<plist/>\n").unwrap();
    assert_eq!(workshop.signed_mach_o(&plain, &team()).await, Ok(()));
}

#[tokio::test]
async fn upd_011_a_valid_ad_hoc_app_passes_the_deep_check_and_fails_the_file_checks() {
    let bench = bench();
    let running = Running::default();
    let workshop = workshop(&bench, &running);
    let mut files: Vec<(&str, &str)> = programs().map(|name| (name, "universal")).collect();
    files.push(("anima-desktop", "universal"));
    let app = app(&bench, "Anima.app", &files);
    fs::write(
        app.join("Contents/Info.plist"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict>\
         <key>CFBundleExecutable</key><string>anima-desktop</string>\
         <key>CFBundleIdentifier</key><string>org.example.anima</string></dict></plist>\n",
    )
    .unwrap();
    let sign = ["-s", "-", "-f", "--options", "runtime"];
    tool(
        "/usr/bin/codesign",
        &[&sign[..], &[app.to_str().unwrap()]].concat(),
        &bench.made,
    );
    // The deep check and the architecture check pass, so only the check of each file refuses it.
    tool(
        "/usr/bin/codesign",
        &["--verify", "--deep", "--strict", app.to_str().unwrap()],
        &bench.made,
    );
    assert_eq!(
        workshop.check_app(&app, &team()).await,
        Err(BuilderError::Bundle)
    );
}
