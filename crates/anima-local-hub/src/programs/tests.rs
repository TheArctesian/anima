use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

mod digest;

use super::{CopyError, are_in_place, copy, put_in_place, remove};
use crate::members::PROGRAMS;
use crate::{Carried, Folders};

/// What the application carries: the programs in one folder and a PostgreSQL tree.
pub(crate) fn carried(folder: &Path, mark: &str) -> Carried {
    carried_in(&folder.join(format!("bundle {mark}")), mark)
}

/// The same, in the folder `bundle`.
pub(crate) fn carried_in(bundle: &Path, mark: &str) -> Carried {
    let (programs, tree) = (bundle.join("MacOS"), bundle.join("database"));
    fs::create_dir_all(&programs).unwrap();
    for name in PROGRAMS {
        program(&programs.join(name), mark);
    }
    fs::create_dir_all(tree.join("lib")).unwrap();
    fs::create_dir_all(tree.join("share/timezone")).unwrap();
    fs::create_dir_all(tree.join("bin")).unwrap();
    for name in [
        "postgres",
        "initdb",
        "pg_ctl",
        "pg_isready",
        "pg_controldata",
    ] {
        program(&tree.join("bin").join(name), mark);
    }
    fs::write(tree.join("lib/libpq.5.17.dylib"), mark).unwrap();
    symlink("libpq.5.17.dylib", tree.join("lib/libpq.5.dylib")).unwrap();
    fs::write(tree.join("share/timezone/UTC"), mark).unwrap();
    Carried::new(programs, tree)
}

fn program(path: &Path, mark: &str) {
    fs::write(path, mark).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn rig() -> (TempDir, Carried, Folders) {
    let folder = tempfile::tempdir().unwrap();
    let carried = carried(folder.path(), "one");
    let folders = Folders::new(folder.path().join("hub root"), folder.path().join("s")).unwrap();
    (folder, carried, folders)
}

fn mode_of(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777
}

fn current(folders: &Folders, file: &str) -> PathBuf {
    folders.programs().join(file)
}

#[test]
fn dbl_013_the_unit_runs_a_copy_that_only_the_user_can_change() {
    let (_folder, carried, folders) = rig();

    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    assert!(are_in_place(&folders));
    for name in PROGRAMS {
        assert_eq!(
            mode_of(&current(&folders, "bin").join(name)),
            0o700,
            "{name}"
        );
    }
    assert_eq!(
        mode_of(&current(&folders, "database-tree/bin/postgres")),
        0o700
    );
    assert_eq!(
        mode_of(&current(&folders, "database-tree/share/timezone/UTC")),
        0o600
    );
    assert_eq!(mode_of(&current(&folders, "database-tree/share")), 0o700);
    assert_eq!(
        fs::read_link(current(&folders, "database-tree/lib/libpq.5.dylib")).unwrap(),
        PathBuf::from("libpq.5.17.dylib")
    );
    assert_eq!(mode_of(&folders.all_programs()), 0o700);
}

#[test]
fn a_copy_does_not_touch_the_programs_that_run_until_it_is_put_in_place() {
    let (folder, carried_one, folders) = rig();
    copy(&carried_one, &folders).unwrap();
    put_in_place(&folders).unwrap();
    let carried_two = carried(folder.path(), "two");

    copy(&carried_two, &folders).unwrap();

    assert_eq!(
        fs::read_to_string(current(&folders, "bin/anima-hub")).unwrap(),
        "one"
    );

    put_in_place(&folders).unwrap();

    assert_eq!(
        fs::read_to_string(current(&folders, "bin/anima-hub")).unwrap(),
        "two"
    );
    assert_eq!(
        fs::read_to_string(current(&folders, "database-tree/bin/postgres")).unwrap(),
        "two"
    );
    let left: Vec<_> = fs::read_dir(folders.all_programs())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["current"]);
}

#[test]
fn a_copy_that_a_crash_left_half_made_is_replaced() {
    let (_folder, carried, folders) = rig();
    fs::create_dir_all(folders.programs_being_copied().join("bin")).unwrap();
    fs::write(
        folders.programs_being_copied().join("bin/anima-hub"),
        "half",
    )
    .unwrap();

    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    assert_eq!(
        fs::read_to_string(current(&folders, "bin/anima-hub")).unwrap(),
        "one"
    );
}

#[test]
fn a_bundle_without_one_of_the_programs_is_refused() {
    let (_folder, carried, folders) = rig();
    fs::remove_file(carried.program("anima-gateway")).unwrap();

    let error = copy(&carried, &folders).unwrap_err();

    assert!(matches!(error, CopyError::NotCarried(_)), "{error}");
    assert!(!are_in_place(&folders));
}

#[test]
fn a_program_that_is_a_link_is_refused() {
    let (folder, carried, folders) = rig();
    fs::remove_file(carried.program("anima-hub")).unwrap();
    symlink(
        folder.path().join("elsewhere"),
        carried.program("anima-hub"),
    )
    .unwrap();

    let error = copy(&carried, &folders).unwrap_err();

    assert!(matches!(error, CopyError::NotCarried(_)), "{error}");
}

#[test]
fn a_link_in_the_tree_that_leaves_its_folder_is_refused() {
    for target in ["/bin/sh", "../bin/postgres", "../../elsewhere"] {
        let (_folder, carried, folders) = rig();
        symlink(target, carried.tree().join("lib/libz.dylib")).unwrap();

        let error = copy(&carried, &folders).unwrap_err();

        assert!(matches!(error, CopyError::Refused(_)), "{target}: {error}");
    }
}

#[test]
fn a_folder_that_is_not_a_postgresql_tree_is_refused() {
    let (_folder, carried, folders) = rig();
    fs::remove_file(carried.tree().join("bin/postgres")).unwrap();

    let error = copy(&carried, &folders).unwrap_err();

    assert!(matches!(error, CopyError::NotATree(_)), "{error}");
}

#[test]
fn remove_takes_the_programs_and_leaves_the_rest_of_the_hub() {
    let (_folder, carried, folders) = rig();
    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();
    fs::write(folders.vault_key(), "kept").unwrap();

    remove(&folders).unwrap();
    remove(&folders).unwrap();

    assert!(!folders.all_programs().exists());
    assert!(!are_in_place(&folders));
    assert_eq!(fs::read_to_string(folders.vault_key()).unwrap(), "kept");
}

#[test]
fn when_the_new_copy_cannot_take_its_place_the_programs_before_stay() {
    let (_folder, carried, folders) = rig();
    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    // No copy was made, so there is nothing to put in place.
    let error = put_in_place(&folders).unwrap_err();

    assert!(matches!(error, CopyError::Io { .. }), "{error}");
    assert!(are_in_place(&folders));
    assert_eq!(
        fs::read_to_string(current(&folders, "bin/anima-hub")).unwrap(),
        "one"
    );
}
