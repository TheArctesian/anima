//! The bundle paths: only the build's own regular files are signed, and the `.sig` file is never
//! written through a link the build left.

use std::{
    fs,
    io::Read,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use super::COMMIT;
use crate::{
    builder::{
        artifact,
        bundled::{Bundle, Bundled},
        error::BuilderError,
        job::{Commit, Target, Version},
        reopened,
    },
    updater_key::prehash,
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
fn bld_045_only_a_regular_file_inside_the_clone_is_the_builds_own() {
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
fn bld_045_a_sig_file_left_as_a_link_is_replaced_and_its_target_kept() {
    let root = tempfile::tempdir().unwrap();
    let updater = root.path().join("Anima.AppImage");
    let target = root.path().join("bashrc");
    fs::write(&target, b"keep me").unwrap();
    symlink(&target, root.path().join("Anima.AppImage.sig")).unwrap();

    artifact::write_signature(&updater, "c2lnbmF0dXJl").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"keep me");
    assert_eq!(artifact::read_signature(&updater).unwrap(), "c2lnbmF0dXJl");

    fs::write(root.path().join("Anima.AppImage.sig"), vec![b'a'; 5000]).unwrap();
    assert_eq!(
        artifact::read_signature(&updater),
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

/// A clone with one bundled file, recorded as the bundle step records it.
fn bundled_clone() -> (tempfile::TempDir, PathBuf, Bundle) {
    let root = tempfile::tempdir().unwrap();
    let clone = root.path().join("clone");
    fs::create_dir_all(clone.join("bundle")).unwrap();
    let path = clone.join("bundle/Anima_0.1.12_amd64.AppImage");
    fs::write(&path, b"image").unwrap();
    let digest = prehash(artifact::open_own(&clone, &path).unwrap()).unwrap();
    let bundle = Bundle {
        commit: Commit::parse(COMMIT).unwrap(),
        target: Target::LinuxX86_64,
        version: Version::parse("0.1.12").unwrap(),
        path,
        digest,
    };
    (root, clone, bundle)
}

#[test]
fn s_3_a_hard_link_to_a_file_outside_the_clone_is_not_signed() {
    let (root, clone, bundle) = bundled_clone();
    let outside = root.path().join("id_ed25519");
    fs::write(&outside, b"a file the build must not get signed").unwrap();
    let linked = clone.join("bundle/linked.AppImage");
    fs::hard_link(&outside, &linked).unwrap();
    assert_eq!(
        artifact::open_own(&clone, &linked).err(),
        Some(BuilderError::Bundle)
    );
    // The bundled file itself, hard-linked after the bundle step, is refused too.
    fs::hard_link(&bundle.path, root.path().join("copy")).unwrap();
    assert_eq!(reopened(&clone, &bundle).err(), Some(BuilderError::Bundle));
}

#[test]
fn s_3_only_the_bytes_the_bundle_step_hashed_are_signed() {
    let (_root, clone, bundle) = bundled_clone();
    let (mut file, digest) = reopened(&clone, &bundle).unwrap();
    assert_eq!(digest, bundle.digest);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        bytes, b"image",
        "the descriptor is handed back at its start"
    );
    fs::write(&bundle.path, b"swapped").unwrap();
    assert_eq!(reopened(&clone, &bundle).err(), Some(BuilderError::Bundle));
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
