use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};

use super::{FileError, Root, copy_path, read, replace};

fn home() -> (tempfile::TempDir, u32) {
    let home = tempfile::tempdir().expect("a temporary home");
    let user = fs::metadata(home.path()).expect("its owner").uid();
    (home, user)
}

#[test]
fn mcp_031_032_a_file_is_replaced_whole_with_its_mode_and_one_copy_of_it_as_it_was() {
    let (home, user) = home();
    let root = Root {
        home: home.path(),
        user,
    };
    let path = home.path().join(".cursor/mcp.json");
    fs::create_dir(home.path().join(".cursor")).expect("folder");
    fs::write(&path, "first").expect("file");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("mode");
    let before = read(root, &path).expect("read");
    assert_eq!(before.as_deref(), Some("first"));
    replace(root, &path, before.as_deref(), "second").expect("replace");
    assert_eq!(fs::read_to_string(&path).expect("new"), "second");
    assert_eq!(
        fs::metadata(&path).expect("mode").permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_to_string(copy_path(&path)).expect("copy"), "first");
    // A second change keeps the first copy: it is the file as it was before Anima.
    replace(root, &path, Some("second"), "third").expect("replace");
    assert_eq!(fs::read_to_string(copy_path(&path)).expect("copy"), "first");
    let names: Vec<_> = fs::read_dir(home.path().join(".cursor"))
        .expect("list")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(names.len(), 2, "no temporary file stays: {names:?}");
}

#[test]
fn mcp_031_a_file_that_is_not_there_is_made_private_with_its_folder_and_no_copy() {
    let (home, user) = home();
    let root = Root {
        home: home.path(),
        user,
    };
    let path = home.path().join(".copilot/mcp-config.json");
    assert!(read(root, &path).expect("read").is_none());
    replace(root, &path, None, "{}").expect("replace");
    assert_eq!(
        fs::metadata(&path).expect("mode").permissions().mode() & 0o777,
        0o600
    );
    assert!(!copy_path(&path).exists());
}

#[test]
fn mcp_034_a_symbolic_link_at_the_file_or_at_a_folder_is_refused_and_its_target_stays() {
    let (home, user) = home();
    let root = Root {
        home: home.path(),
        user,
    };
    let elsewhere = home.path().join("elsewhere.json");
    fs::write(&elsewhere, "theirs").expect("target");
    fs::create_dir(home.path().join(".cursor")).expect("folder");
    let path = home.path().join(".cursor/mcp.json");
    symlink(&elsewhere, &path).expect("link");
    assert!(matches!(read(root, &path), Err(FileError::Link)));
    assert!(matches!(
        replace(root, &path, None, "ours"),
        Err(FileError::Link)
    ));
    assert_eq!(fs::read_to_string(&elsewhere).expect("target"), "theirs");

    fs::create_dir(home.path().join("real")).expect("folder");
    symlink(home.path().join("real"), home.path().join(".config")).expect("link");
    let inside = home.path().join(".config/zed/settings.json");
    assert!(matches!(
        replace(root, &inside, None, "ours"),
        Err(FileError::Link)
    ));
    assert!(!home.path().join("real/zed").exists());
}

#[test]
fn mcp_034_a_file_or_a_folder_that_others_can_write_and_a_file_too_large_are_refused() {
    let (home, user) = home();
    let root = Root {
        home: home.path(),
        user,
    };
    fs::create_dir(home.path().join(".cursor")).expect("folder");
    let path = home.path().join(".cursor/mcp.json");
    fs::write(&path, "{}").expect("file");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).expect("mode");
    assert!(matches!(read(root, &path), Err(FileError::Shared)));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("mode");
    fs::set_permissions(
        home.path().join(".cursor"),
        fs::Permissions::from_mode(0o777),
    )
    .expect("mode");
    assert!(matches!(
        replace(root, &path, None, "ours"),
        Err(FileError::Shared)
    ));
    fs::set_permissions(
        home.path().join(".cursor"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("mode");
    assert!(matches!(
        read(
            Root {
                home: home.path(),
                user: user + 1
            },
            &path
        ),
        Err(FileError::Shared)
    ));
    fs::write(&path, vec![b' '; 1024 * 1024 + 1]).expect("large");
    assert!(matches!(read(root, &path), Err(FileError::TooLarge)));
    assert!(matches!(
        read(root, &home.path().join(".cursor")),
        Err(FileError::NotAFile)
    ));
    assert!(matches!(
        read(root, std::path::Path::new("/etc/hosts")),
        Err(FileError::NotAFile)
    ));
}
