//! The Linux token file: mode 0600, owned by the user, replaced by rename (MET-013, K8 Section 2).

use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

use anima_agent::config::home::Home;
use anima_agent::config::private_file::PrivateFileError;
use anima_agent::token_store::{Credential, TokenStore, TokenStoreError};

use crate::kit::{at, token};

fn credential(seed: u8, days: i64) -> Credential {
    Credential {
        token: token(seed),
        expires_at: at(0) + chrono::TimeDelta::days(days),
        hub: None,
    }
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .expect("a folder")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    found
}

#[test]
fn met_013_the_token_file_is_0600_owned_by_the_user_and_holds_the_expiry() {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    store.save(&credential(1, 365)).expect("stored");
    let metadata = fs::metadata(home.token_path()).expect("a file");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(
        metadata.uid(),
        fs::metadata("/proc/self").expect("self").uid()
    );
    assert_eq!(
        fs::metadata(home.dir())
            .expect("folder")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let loaded = store.load().expect("reads").expect("a token");
    assert_eq!(loaded.token.expose(), token(1).expose());
    assert_eq!(loaded.expires_at, at(0) + chrono::TimeDelta::days(365));
}

#[test]
fn met_013_a_new_token_replaces_the_file_by_rename_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    store.save(&credential(1, 365)).expect("stored");
    let first = fs::metadata(home.token_path()).expect("a file").ino();
    store.save(&credential(2, 365)).expect("stored again");
    let second = fs::metadata(home.token_path()).expect("a file").ino();
    assert_ne!(
        first, second,
        "a rename gives the file a new inode; a rewrite keeps it"
    );
    assert_eq!(names(&dir.path().join("agent")), ["token"]);
    assert_eq!(
        fs::metadata(home.token_path())
            .expect("a file")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let loaded = store.load().expect("reads").expect("a token");
    assert_eq!(loaded.token.expose(), token(2).expose());
}

#[test]
fn met_013_a_token_file_others_can_read_is_refused() {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    store.save(&credential(1, 365)).expect("stored");
    fs::set_permissions(home.token_path(), fs::Permissions::from_mode(0o644)).expect("chmod");
    assert!(matches!(
        store.load(),
        Err(TokenStoreError::File(PrivateFileError::Exposed))
    ));
}

#[test]
fn met_013_a_store_with_no_token_is_empty_and_a_damaged_one_is_an_error() {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    assert!(store.load().expect("reads").is_none());
    store.save(&credential(1, 1)).expect("stored");
    fs::write(home.token_path(), b"{}").expect("write");
    fs::set_permissions(home.token_path(), fs::Permissions::from_mode(0o600)).expect("chmod");
    assert!(matches!(store.load(), Err(TokenStoreError::Corrupt)));
}
