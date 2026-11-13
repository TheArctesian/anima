//! The bundle paths: only the build's own regular files are uploaded, and only while their bytes
//! are the ones the bundle step recorded (D-13, D-22).

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use super::COMMIT;
use crate::builder::{
    artifact,
    bundled::{Bundle, Bundled},
    error::BuilderError,
    job::{Commit, Target, Version},
    upload::{bound_updater, sha256_of},
};

#[test]
fn bld_043_the_updater_file_is_where_tauri_bundles_it() {
    let clone = Path::new("/data/builds/anima");
    let version = Version::parse("0.1.12").unwrap();
    assert_eq!(
        artifact::updater_file(clone, Target::LinuxX86_64, &version),
        Path::new("/data/builds/anima/target/release/bundle/appimage/Anima_0.1.12_amd64.AppImage")
    );
    assert_eq!(
        artifact::updater_file(clone, Target::DarwinUniversal, &version),
        Path::new(
            "/data/builds/anima/target/universal-apple-darwin/release/bundle/macos/Anima.app.tar.gz"
        )
    );
}

#[test]
fn bld_043_only_a_regular_file_inside_the_clone_is_the_builds_own() {
    let root = tempfile::tempdir().unwrap();
    let clone = root.path().join("clone");
    fs::create_dir_all(clone.join("bundle")).unwrap();
    let file = clone.join("bundle/Anima.AppImage");
    fs::write(&file, b"image").unwrap();
    let outside = root.path().join("secret");
    fs::write(&outside, b"not the build's").unwrap();
    let link = clone.join("bundle/link.AppImage");
    symlink(&outside, &link).unwrap();
    symlink(root.path(), clone.join("escape")).unwrap();

    assert!(artifact::own_file(&clone, &file).is_ok());
    assert_eq!(artifact::own_file(&clone, &link), Err(BuilderError::Bundle));
    assert_eq!(
        artifact::own_file(&clone, &clone.join("escape/secret")),
        Err(BuilderError::Bundle)
    );
    assert_eq!(
        artifact::own_file(&clone, &clone.join("bundle")),
        Err(BuilderError::Bundle)
    );
    assert_eq!(
        artifact::own_file(&clone, &clone.join("missing")),
        Err(BuilderError::Bundle)
    );
}

#[test]
fn bld_047_lipo_must_name_both_architectures() {
    let lines = |text: &str| vec![text.to_owned()];
    assert!(artifact::universal(&lines("x86_64 arm64")));
    assert!(artifact::universal(&lines("arm64 x86_64")));
    assert!(!artifact::universal(&lines("arm64")));
    assert!(!artifact::universal(&lines("x86_64h arm64e")));
    assert!(!artifact::universal(&[]));
}

#[test]
fn bld_085_only_apples_final_accepted_status_counts_as_notarized() {
    let lines = |text: &str| text.lines().map(str::to_owned).collect::<Vec<_>>();
    let accepted = "Waiting for processing to complete.\nCurrent status: Accepted....\nProcessing complete\n  id: 0b1c\n  status: Accepted";
    assert!(artifact::notarized(&lines(accepted)));
    let invalid =
        "Current status: Accepted....\nProcessing complete\n  id: 0b1c\n  status: Invalid";
    assert!(!artifact::notarized(&lines(invalid)));
    assert!(!artifact::notarized(&lines("  status: In Progress")));
    assert!(!artifact::notarized(&[]));
}

/// A clone with one bundled file, recorded as the bundle step records it.
fn bundled_clone() -> (tempfile::TempDir, PathBuf, Bundle) {
    let root = tempfile::tempdir().unwrap();
    let clone = root.path().join("clone");
    fs::create_dir_all(clone.join("bundle")).unwrap();
    let path = clone.join("bundle/Anima_0.1.12_amd64.AppImage");
    fs::write(&path, b"image").unwrap();
    let sha256 = sha256_of(artifact::open_own(&clone, &path).unwrap()).unwrap();
    let bundle = Bundle {
        commit: Commit::parse(COMMIT).unwrap(),
        target: Target::LinuxX86_64,
        version: Version::parse("0.1.12").unwrap(),
        path,
        sha256,
    };
    (root, clone, bundle)
}

#[test]
fn s_3_a_hard_link_to_a_file_outside_the_clone_is_not_uploaded() {
    let (root, clone, bundle) = bundled_clone();
    let outside = root.path().join("id_ed25519");
    fs::write(&outside, b"a file the build must not get uploaded").unwrap();
    let linked = clone.join("bundle/linked.AppImage");
    fs::hard_link(&outside, &linked).unwrap();
    assert_eq!(
        artifact::open_own(&clone, &linked).err(),
        Some(BuilderError::Bundle)
    );
    // The bundled file itself, hard-linked after the bundle step, is refused too.
    fs::hard_link(&bundle.path, root.path().join("copy")).unwrap();
    assert_eq!(
        bound_updater(&clone, &bundle).err(),
        Some(BuilderError::Bundle)
    );
}

#[test]
fn s_3_only_the_bytes_the_bundle_step_hashed_are_uploaded() {
    let (_root, clone, bundle) = bundled_clone();
    assert_eq!(bound_updater(&clone, &bundle).unwrap(), b"image");
    fs::write(&bundle.path, b"swapped").unwrap();
    assert_eq!(
        bound_updater(&clone, &bundle).err(),
        Some(BuilderError::Bundle)
    );
}

#[test]
fn s_3_the_record_names_one_job_and_the_next_checkout_forgets_it() {
    let (_root, _clone, bundle) = bundled_clone();
    let bundled = Bundled::default();
    let version = Version::parse("0.1.12").unwrap();
    assert_eq!(bundled.of(Target::LinuxX86_64, &version), None);
    bundled.record(bundle.clone());
    assert_eq!(bundled.of(Target::LinuxX86_64, &version), Some(bundle));
    assert_eq!(bundled.of(Target::DarwinUniversal, &version), None);
    assert_eq!(
        bundled.of(Target::LinuxX86_64, &Version::parse("0.1.13").unwrap()),
        None
    );
    bundled.forget();
    assert_eq!(bundled.of(Target::LinuxX86_64, &version), None);
}
