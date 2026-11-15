use std::fs::{self, Permissions};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::{appended_file, folder, new_file};

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn db_003_a_new_folder_and_its_new_parents_are_owner_only() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("a/b");

    folder(&path).unwrap();

    assert_eq!(mode_of(&path), 0o700);
    assert_eq!(mode_of(&parent.path().join("a")), 0o700);
}

#[test]
fn db_003_a_folder_that_was_open_to_others_is_closed() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("wide");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o755)).unwrap();

    folder(&path).unwrap();

    assert_eq!(mode_of(&path), 0o700);
}

#[test]
fn db_003_a_symbolic_link_is_refused_as_a_folder() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("elsewhere");
    fs::create_dir(&target).unwrap();
    fs::set_permissions(&target, Permissions::from_mode(0o755)).unwrap();
    let link = parent.path().join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let error = folder(&link).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(mode_of(&target), 0o755);
}

#[test]
fn db_002_a_new_file_is_owner_only_and_holds_nothing_of_a_file_left_there() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("secret");
    fs::write(&path, "left by a crash").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o644)).unwrap();

    drop(new_file(&path).unwrap());

    assert_eq!(mode_of(&path), 0o600);
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
}

#[test]
fn db_002_a_new_file_does_not_write_through_a_planted_link() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("elsewhere");
    fs::write(&target, "untouched").unwrap();
    let path = parent.path().join("secret");
    std::os::unix::fs::symlink(&target, &path).unwrap();

    {
        use std::io::Write;
        new_file(&path).unwrap().write_all(b"a secret").unwrap();
    }

    assert_eq!(fs::read_to_string(&target).unwrap(), "untouched");
    assert!(!fs::symlink_metadata(&path).unwrap().is_symlink());
}

#[test]
fn an_appended_file_keeps_its_text_and_is_closed_to_others() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("log");
    fs::write(&path, "first\n").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o644)).unwrap();

    {
        use std::io::Write;
        appended_file(&path)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
    }

    assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
    assert_eq!(mode_of(&path), 0o600);
}
