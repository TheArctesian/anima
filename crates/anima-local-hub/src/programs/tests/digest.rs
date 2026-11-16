//! The digest of a carried set decides an update (D-6).

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

use super::{carried, carried_in, current, mode_of, rig};
use crate::Carried;
use crate::programs::digest::{self, Digest};
use crate::programs::{copy, put_in_place};

fn of(carried: &Carried) -> Digest {
    digest::of(carried).unwrap()
}

#[test]
fn the_digest_of_the_source_is_the_digest_of_the_copy_that_install_stores() {
    let (_folder, carried, folders) = rig();

    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    assert!(of(&carried).is_stored_in(&folders));
    assert_eq!(
        of(&carried),
        of(&Carried::installed_in(&folders.programs()))
    );
    assert_eq!(of(&carried).hex().len(), 64);
    let stored = current(&folders, "digest");
    assert_eq!(
        fs::read_to_string(&stored).unwrap(),
        format!("{}\n", of(&carried).hex())
    );
    assert_eq!(mode_of(&stored), 0o600);
}

#[test]
fn a_copy_without_a_digest_file_is_not_the_digest_of_the_source() {
    let (_folder, carried, folders) = rig();
    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    fs::remove_file(current(&folders, "digest")).unwrap();

    assert!(!of(&carried).is_stored_in(&folders));
}

/// A change of the source, with what it is.
type Change = (&'static str, fn(&Carried));

fn changes() -> Vec<Change> {
    vec![
        ("one byte of a program", |carried| {
            fs::write(carried.program("anima-gateway"), "onf").unwrap();
        }),
        ("one byte of a tree file", |carried| {
            fs::write(carried.tree().join("share/timezone/UTC"), "onf").unwrap();
        }),
        ("the target of a link", |carried| {
            let link = carried.tree().join("lib/libpq.5.dylib");
            fs::remove_file(&link).unwrap();
            symlink("libpq.5.18.dylib", &link).unwrap();
        }),
        ("the executable bit of a tree file", |carried| {
            let file = carried.tree().join("bin/pg_ctl");
            fs::set_permissions(file, fs::Permissions::from_mode(0o644)).unwrap();
        }),
        ("a new file", |carried| {
            fs::write(carried.tree().join("share/new"), "").unwrap();
        }),
        ("a new folder", |carried| {
            fs::create_dir(carried.tree().join("share/empty")).unwrap();
        }),
        ("a file that is now a link", |carried| {
            let file = carried.tree().join("share/timezone/UTC");
            fs::remove_file(&file).unwrap();
            symlink("Zulu", file).unwrap();
        }),
    ]
}

#[test]
fn each_change_of_the_carried_set_gives_another_digest() {
    for (change, make) in changes() {
        let (_folder, carried, folders) = rig();
        copy(&carried, &folders).unwrap();
        put_in_place(&folders).unwrap();

        make(&carried);

        assert!(!of(&carried).is_stored_in(&folders), "{change}");
    }
}

#[test]
fn a_program_that_the_copy_makes_executable_is_the_same_whatever_the_mode_of_its_source() {
    let (_folder, carried, folders) = rig();
    fs::set_permissions(
        carried.program("anima-hub"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    assert!(of(&carried).is_stored_in(&folders));
}

#[test]
fn files_that_the_copy_does_not_carry_are_not_in_the_digest() {
    let (_folder, carried, folders) = rig();
    copy(&carried, &folders).unwrap();
    put_in_place(&folders).unwrap();

    // The application's own program sits in the same folder as the five.
    fs::write(carried.programs().join("anima-desktop"), "other").unwrap();

    assert!(of(&carried).is_stored_in(&folders));
}

#[test]
fn entries_that_are_made_in_another_order_give_the_same_digest() {
    let folder = tempfile::tempdir().unwrap();
    let names = ["b", "a", "c", "ab", "a b", "z"];
    let (one, two) = (
        carried_in(&folder.path().join("one"), "m"),
        carried_in(&folder.path().join("two"), "m"),
    );
    let add = |carried: &Carried, names: &mut dyn Iterator<Item = &&str>| {
        for name in names {
            fs::write(carried.tree().join("share").join(name), name).unwrap();
        }
    };

    add(&one, &mut names.iter());
    add(&two, &mut names.iter().rev());

    assert_eq!(of(&one), of(&two));
}

#[test]
fn two_sets_that_differ_only_in_where_a_name_ends_do_not_give_one_digest() {
    let folder = tempfile::tempdir().unwrap();
    let one = carried_in(&folder.path().join("one"), "m");
    let two = carried_in(&folder.path().join("two"), "m");
    fs::write(one.tree().join("share/ab"), "c").unwrap();
    fs::write(two.tree().join("share/a"), "bc").unwrap();

    assert_ne!(of(&one), of(&two));
}

#[test]
fn the_digest_refuses_what_the_copy_refuses() {
    use crate::programs::CopyError;
    type Refusal = (&'static str, fn(&Path));
    let refusals: [Refusal; 3] = [
        ("a link that leaves its folder", |tree| {
            symlink("/bin/sh", tree.join("lib/libz.dylib")).unwrap();
        }),
        ("a program that is a link", |tree| {
            let program = tree.parent().unwrap().join("MacOS/anima-hub");
            fs::remove_file(&program).unwrap();
            symlink("elsewhere", program).unwrap();
        }),
        ("a folder deeper than a tree is", |tree| {
            let mut deep = tree.to_path_buf();
            for step in 0..20 {
                deep.push(format!("d{step}"));
            }
            fs::create_dir_all(deep).unwrap();
        }),
    ];
    for (what, make) in refusals {
        let folder = tempfile::tempdir().unwrap();
        let carried = carried(folder.path(), "one");
        make(carried.tree());

        let error = digest::of(&carried).unwrap_err();

        assert!(
            matches!(
                error,
                CopyError::Refused(_) | CopyError::NotCarried(_) | CopyError::TooLarge
            ),
            "{what}: {error}"
        );
    }
}

#[test]
fn the_name_of_an_entry_is_part_of_the_digest() {
    let folder = tempfile::tempdir().unwrap();
    let one = carried_in(&folder.path().join("one"), "m");
    let two = carried_in(&folder.path().join("two"), "m");
    fs::rename(
        two.tree().join("share/timezone/UTC"),
        two.tree().join("share/timezone/Zulu"),
    )
    .unwrap();

    assert_ne!(of(&one), of(&two));
}

#[test]
fn two_files_that_swap_their_contents_give_another_digest() {
    let folder = tempfile::tempdir().unwrap();
    let one = carried_in(&folder.path().join("one"), "m");
    let two = carried_in(&folder.path().join("two"), "m");
    for set in [&one, &two] {
        fs::write(set.tree().join("share/a"), "first").unwrap();
        fs::write(set.tree().join("share/b"), "second").unwrap();
    }
    fs::write(two.tree().join("share/a"), "second").unwrap();
    fs::write(two.tree().join("share/b"), "first").unwrap();

    assert_ne!(of(&one), of(&two));
}
