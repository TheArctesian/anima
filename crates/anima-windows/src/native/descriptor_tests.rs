use crate::files;
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
};

#[test]
fn protected_files_are_readable_and_public_tokens_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("private");
    files::directory(&folder).unwrap();
    assert!(files::secure(&folder, true).unwrap());
    let path = folder.join("token");
    fs::write(&path, "test-value").unwrap();
    assert!(files::read(&path, true).is_ok());
    let handle = OpenOptions::new()
        .access_mode(0x0006_0000)
        .open(&path)
        .unwrap();
    let user = super::Identity::current().unwrap().text().unwrap();
    super::set_acl(&handle, &format!("D:P(A;;FA;;;{user})(A;;FR;;;WD)")).unwrap();
    assert!(!files::secure(&path, true).unwrap());
    assert!(files::read(&path, true).is_err());
    assert!(files::read(&path, false).is_ok());
    let replacement = folder.join("replacement");
    fs::write(&replacement, "").unwrap();
    files::copy_acl(&path, &replacement).unwrap();
    assert!(!files::secure(&replacement, true).unwrap());
    assert!(files::secure(&replacement, false).unwrap());
    super::set_acl(&handle, &format!("D:P(A;;FA;;;{user})(A;;FW;;;WD)")).unwrap();
    assert!(files::read(&path, false).is_err());
    assert!(files::copy_acl(&path, &replacement).is_err());
}

#[test]
fn directories_and_reparse_points_are_not_read_as_files() {
    let root = tempfile::tempdir().unwrap();
    assert!(files::read(root.path(), false).is_err());
    let target = root.path().join("target");
    fs::write(&target, "test-value").unwrap();
    let link = root.path().join("link");
    // Developer mode or the link privilege is needed to create this fixture.
    if std::os::windows::fs::symlink_file(&target, &link).is_ok() {
        assert!(files::read(&link, false).is_err());
        assert!(files::protect(&link).is_err());
    }
}
