use std::fs;
use std::os::unix::fs::PermissionsExt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::{KeyError, prepare};

#[test]
fn a_new_database_gets_a_key_of_32_bytes_that_only_the_user_can_read() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vault.key");

    prepare(&path, true).unwrap();

    let key = STANDARD.decode(fs::read(&path).unwrap()).unwrap();
    assert_eq!(key.len(), 32);
    assert_ne!(key, [0; 32]);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 1);
}

#[test]
fn a_key_that_is_there_is_never_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vault.key");
    prepare(&path, true).unwrap();
    let first = fs::read(&path).unwrap();

    prepare(&path, true).unwrap();
    prepare(&path, false).unwrap();

    assert_eq!(fs::read(&path).unwrap(), first);
}

#[test]
fn no_key_is_made_for_a_database_that_an_older_key_sealed() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vault.key");

    let error = prepare(&path, false).unwrap_err();

    assert!(matches!(error, KeyError::Missing(_)), "{error}");
    assert!(!path.exists());
}

#[test]
fn a_link_at_the_keys_path_is_refused_and_not_followed() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("vault.key");
    fs::write(folder.path().join("elsewhere"), "open to others").unwrap();
    std::os::unix::fs::symlink(folder.path().join("elsewhere"), &path).unwrap();

    for database_is_new in [true, false] {
        let error = prepare(&path, database_is_new).unwrap_err();

        assert!(matches!(error, KeyError::NotAFile(_)), "{error}");
    }
    assert_eq!(
        fs::read_to_string(folder.path().join("elsewhere")).unwrap(),
        "open to others"
    );
}

#[test]
fn two_keys_are_not_the_same() {
    let folder = tempfile::tempdir().unwrap();
    let (first, second) = (folder.path().join("a"), folder.path().join("b"));

    prepare(&first, true).unwrap();
    prepare(&second, true).unwrap();

    assert_ne!(fs::read(first).unwrap(), fs::read(second).unwrap());
}
