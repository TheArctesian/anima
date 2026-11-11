//! Whether the hub can record now (K4 §1 step 2, D-14): a root that is set, a folder,
//! writable, with 1 GiB free.

use std::{io, os::unix::fs::PermissionsExt as _, path::Path};

use super::{FAILING, PLENTY};
use crate::sessions::{
    Recording,
    recording::{
        can_record,
        space::{CannotRecord, MIN_FREE, check, free_space},
    },
};

fn at(root: &Path) -> Recording {
    Recording {
        root: Some(root.to_owned()),
        max_bytes: 1,
    }
}

#[test]
fn d_14_a_writable_folder_with_1_gib_free_can_record() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(check(&at(root.path()), PLENTY), Ok(()));
    assert_eq!(
        check(&at(root.path()), |_| Ok(MIN_FREE)),
        Ok(()),
        "exactly 1 GiB"
    );
}

#[test]
fn d_14_no_root_cannot_record() {
    let unset = Recording {
        root: None,
        max_bytes: 1,
    };
    assert_eq!(check(&unset, PLENTY), Err(CannotRecord::Unset));
}

#[test]
fn d_14_a_missing_root_or_a_file_cannot_record() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing");
    assert_eq!(
        check(&at(&missing), PLENTY),
        Err(CannotRecord::Unreadable(io::ErrorKind::NotFound))
    );
    let file = root.path().join("file");
    std::fs::write(&file, b"").unwrap();
    assert_eq!(check(&at(&file), PLENTY), Err(CannotRecord::NotFolder));
}

#[test]
fn d_14_a_read_only_root_cannot_record() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let refused = check(&at(root.path()), PLENTY);
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(refused, Err(CannotRecord::NotWritable));
}

#[test]
fn d_14_less_than_1_gib_free_cannot_record() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        check(&at(root.path()), |_| Ok(MIN_FREE - 1)),
        Err(CannotRecord::LowSpace)
    );
    assert_eq!(
        check(&at(root.path()), FAILING),
        Err(CannotRecord::Unreadable(io::ErrorKind::Other))
    );
}

#[test]
fn g_3_free_space_reads_the_file_system() {
    let root = tempfile::tempdir().unwrap();
    // The test folder has more than 1 GiB free; the hub's own reading says so.
    assert_eq!(can_record(&at(root.path())), Ok(()));
    assert!(free_space(root.path()).unwrap() > 0);
    assert!(free_space(&root.path().join("missing")).is_err());
}

#[test]
fn log_007_refusals_name_no_path() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing");
    let text = check(&at(&missing), PLENTY).unwrap_err().to_string();
    assert!(!text.contains(root.path().to_str().unwrap()), "{text}");
}
