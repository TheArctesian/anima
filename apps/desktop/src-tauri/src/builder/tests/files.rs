//! The bundle paths: only the build's own regular files are signed, and the `.sig` file is never
//! written through a link the build left.

use std::{fs, os::unix::fs::symlink, path::Path};

use crate::builder::{
    artifact,
    error::BuilderError,
    job::{Target, Version},
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
