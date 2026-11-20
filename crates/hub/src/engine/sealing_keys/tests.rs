use base64::{Engine as _, engine::general_purpose::STANDARD};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

use super::*;

fn write_key(folder: &Path, name: &str, byte: u8) {
    std::fs::write(
        folder.join(format!("{name}.key")),
        STANDARD.encode([byte; KEY_LEN]),
    )
    .unwrap();
}

/// The identifier the credential vault gives a key of 32 bytes `byte`.
fn credential_id(byte: u8) -> String {
    key_id(&[byte; KEY_LEN])
}

#[test]
fn vlt_003_start_refuses_a_data_key_that_is_the_credential_key() {
    let folder = tempfile::tempdir().unwrap();
    write_key(folder.path(), "engine", 7);

    let refused = SealingKeys::open(Some(folder.path()), credential_id(7).as_bytes());

    assert!(matches!(refused, Err(SealingKeyError::CredentialKey { name }) if name == "engine"));
    let other = SealingKeys::open(Some(folder.path()), credential_id(8).as_bytes());
    assert!(other.is_ok());
}

#[test]
fn vlt_003_start_refuses_two_data_keys_that_are_the_same_key() {
    let folder = tempfile::tempdir().unwrap();
    write_key(folder.path(), "locations", 5);
    write_key(folder.path(), "engine", 5);

    let refused = SealingKeys::open(Some(folder.path()), credential_id(1).as_bytes());

    assert!(matches!(
        refused,
        Err(SealingKeyError::SameKey { first, second }) if first == "engine" && second == "locations"
    ));
}

#[test]
fn vlt_003_a_key_is_made_at_first_use_for_the_hub_user_only_and_never_written_again() {
    let folder = tempfile::tempdir().unwrap();
    let keys = SealingKeys::open(Some(&folder.path().join("data-keys")), b"none").unwrap();

    let first = keys.key("wardrobe").unwrap();
    let path = folder.path().join("data-keys/wardrobe.key");
    #[cfg(unix)]
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    let again = keys.key("wardrobe").unwrap();

    #[cfg(unix)]
    assert_eq!(mode & 0o777, 0o600);
    #[cfg(windows)]
    assert!(anima_windows::files::secure(&path, true).unwrap());
    assert_eq!(first.id(), again.id());
    let sealed = first.seal("files.piece:1", b"plain");
    assert_eq!(again.open("files.piece:1", &sealed).unwrap(), b"plain");
    let leftovers: Vec<_> = std::fs::read_dir(folder.path().join("data-keys"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(leftovers.len(), 1, "{leftovers:?}");
}

#[test]
fn vlt_003_with_no_folder_a_key_is_unavailable() {
    let keys = SealingKeys::open(None, b"id").unwrap();

    assert!(!keys.available());
    assert!(matches!(
        keys.key("engine"),
        Err(SealingKeyError::Unavailable)
    ));
}

#[test]
fn vlt_003_a_name_that_could_be_a_path_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let keys = SealingKeys::open(Some(folder.path()), b"id").unwrap();

    for name in ["../vault", "Engine", "", "a/b", "a.key", &"a".repeat(41)] {
        assert!(
            matches!(keys.key(name), Err(SealingKeyError::BadName)),
            "{name}"
        );
    }
}

#[test]
fn vlt_003_a_value_opens_only_with_its_key_and_its_context() {
    let folder = tempfile::tempdir().unwrap();
    write_key(folder.path(), "engine", 1);
    write_key(folder.path(), "locations", 2);
    let keys = SealingKeys::open(Some(folder.path()), b"id").unwrap();
    let engine = keys.key("engine").unwrap();
    let locations = keys.key("locations").unwrap();

    let sealed = engine.seal("locations.sample:1", b"52.1,4.3");

    assert_eq!(
        engine.open("locations.sample:1", &sealed).unwrap(),
        b"52.1,4.3"
    );
    assert!(matches!(
        engine.open("locations.sample:2", &sealed),
        Err(SealingKeyError::Damaged)
    ));
    assert!(matches!(
        locations.open("locations.sample:1", &sealed),
        Err(SealingKeyError::WrongKey)
    ));
    for cut in 0..sealed.len() {
        assert!(engine.open("locations.sample:1", &sealed[..cut]).is_err());
    }
}

#[test]
fn log_007_debug_and_errors_show_no_key_bytes() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("engine.key"), "c2VjcmV0LXZhbHVl").unwrap();
    let keys = SealingKeys::open(None, b"id").unwrap();
    let refused = SealingKeys::open(Some(folder.path()), b"id").unwrap_err();
    write_key(folder.path(), "engine", 3);
    let key = SealingKeys::open(Some(folder.path()), b"id")
        .unwrap()
        .key("engine")
        .unwrap();

    let shown = format!("{key:?} {keys:?} {refused} {refused:?}");

    assert!(
        !shown.contains(&STANDARD.encode([3_u8; KEY_LEN])),
        "{shown}"
    );
    assert!(!shown.contains("c2VjcmV0"), "{shown}");
    assert!(!shown.contains("[3, 3"), "{shown}");
}
