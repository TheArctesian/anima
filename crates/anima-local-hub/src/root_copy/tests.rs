use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use tempfile::TempDir;

use super::list::Kind;
use super::{RootCopyError, entries};
use crate::Folders;

/// A hub root as a stopped hub leaves it, without a real database: the files only.
fn root() -> (TempDir, Folders) {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("hub");
    let folders = Folders::new(root.clone(), folder.path().join("s")).unwrap();
    let cluster = folders.database().cluster();
    fs::create_dir_all(cluster.join("pg_notify")).unwrap();
    fs::write(cluster.join("PG_VERSION"), "17\n").unwrap();
    fs::write(root.join("vault.key"), "a key").unwrap();
    fs::set_permissions(root.join("vault.key"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::create_dir_all(root.join("link")).unwrap();
    fs::write(root.join("link/hub.json"), "{}").unwrap();
    std::os::unix::fs::symlink("hub.json", root.join("link/now")).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.join("link/control.sock")).unwrap();
    fs::create_dir_all(root.join("programs/current/bin")).unwrap();
    fs::write(root.join("programs/current/bin/anima-hub"), "a program").unwrap();
    fs::write(root.join("lock"), "").unwrap();
    fs::write(root.join("change.lock"), "").unwrap();
    (folder, folders)
}

fn names(list: &super::List) -> Vec<String> {
    list.entries
        .iter()
        .map(|entry| entry.path.display().to_string())
        .collect()
}

#[test]
fn rol_054_a_copy_holds_each_file_of_the_root_but_the_programs_the_locks_and_the_sockets() {
    let (folder, folders) = root();
    let to = folder.path().join("files");

    let list = entries::copy(&folders, &to).unwrap();

    let names = names(&list);
    for kept in [
        "vault.key",
        "link/hub.json",
        "link/now",
        "database/cluster/PG_VERSION",
        "database/cluster/pg_notify",
    ] {
        assert!(names.iter().any(|name| name == kept), "{kept} in {names:?}");
    }
    for left in ["programs", "lock", "change.lock", "link/control.sock"] {
        assert!(!names.iter().any(|name| name.starts_with(left)), "{left}");
        assert!(fs::symlink_metadata(to.join(left)).is_err(), "{left}");
    }
    assert_eq!(fs::read(to.join("vault.key")).unwrap(), b"a key");
    assert!(
        to.join("database/cluster/pg_notify").is_dir(),
        "an empty folder stays"
    );
    assert_eq!(list.cluster, Path::new("database/cluster"));
    assert!(list.has_the_parts_of_a_hub().is_ok());
}

#[test]
fn rol_054_a_copy_keeps_the_mode_of_a_secret_file() {
    let (folder, folders) = root();
    let to = folder.path().join("files");

    let list = entries::copy(&folders, &to).unwrap();

    let key = list
        .entries
        .iter()
        .find(|entry| entry.path == Path::new("vault.key"))
        .unwrap();
    assert_eq!(key.mode, 0o600);
    let copied = fs::metadata(to.join("vault.key"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(copied & 0o777, 0o600);
    assert!(matches!(&key.kind, Kind::File { bytes: 5, sha256 } if sha256.len() == 64));
}

/// Each change that makes a copy useless for a way back, and the words of its fault.
#[test]
fn rol_054_the_check_of_an_entry_finds_a_file_that_is_gone_changed_or_opened_to_others() {
    type Change = (&'static str, fn(&Path), &'static str);
    let changes: [Change; 4] = [
        (
            "vault.key",
            |file| fs::remove_file(file).unwrap(),
            "is not there",
        ),
        (
            "vault.key",
            |file| fs::write(file, "b key").unwrap(),
            "different content",
        ),
        (
            "vault.key",
            |file| fs::write(file, "a longer key").unwrap(),
            "different size",
        ),
        (
            "vault.key",
            |file| fs::set_permissions(file, fs::Permissions::from_mode(0o644)).unwrap(),
            "different mode",
        ),
    ];
    for (name, change, words) in changes {
        let (folder, folders) = root();
        let to = folder.path().join("files");
        let list = entries::copy(&folders, &to).unwrap();
        for entry in &list.entries {
            entries::verify(&to, entry).unwrap();
        }

        change(&to.join(name));

        let fault = list
            .entries
            .iter()
            .find_map(|entry| entries::verify(&to, entry).err())
            .expect("a fault");
        let RootCopyError::Damaged(text) = fault else {
            panic!("{fault:?}");
        };
        assert!(text.contains(name) && text.contains(words), "{text}");
        assert!(
            !text.contains("key\u{0}") && !text.contains("a key"),
            "no content"
        );
    }
}

#[test]
fn rol_054_a_list_without_the_cluster_or_the_vault_key_is_not_a_copy_of_a_hub() {
    let (folder, folders) = root();
    let list = entries::copy(&folders, &folder.path().join("files")).unwrap();

    for part in ["vault.key", "database/cluster/PG_VERSION"] {
        let mut without = list.clone();
        without
            .entries
            .retain(|entry| entry.path != Path::new(part));
        assert!(matches!(
            without.has_the_parts_of_a_hub(),
            Err(RootCopyError::Damaged(text)) if text.contains(part)
        ));
    }
    let mut leaves = list;
    leaves.entries[0].path = "../outside".into();
    assert!(matches!(
        leaves.has_the_parts_of_a_hub(),
        Err(RootCopyError::Damaged(text)) if text.contains("leaves the copy")
    ));
}

#[test]
fn rol_054_no_copy_is_made_of_a_folder_that_holds_no_hub_or_into_a_place_that_is_taken() {
    let folder = tempfile::tempdir().unwrap();
    let empty = Folders::new(folder.path().join("none"), folder.path().join("s")).unwrap();
    let carried = crate::Carried::new(folder.path().into(), folder.path().into());
    assert!(matches!(
        super::make(&carried, &empty, &folder.path().join("copy")),
        Err(RootCopyError::NoHub)
    ));
    assert!(!folder.path().join("none").exists(), "nothing was made");

    let (taken, folders) = root();
    assert!(matches!(
        super::make(&carried, &folders, taken.path()),
        Err(RootCopyError::PlaceTaken)
    ));
}
