//! Windows installation paths and private key export at the shell boundary.
use super::{LocalHubError, carried, environment, key_copy};
use crate::carried::DevFolders;
use std::{ffi::OsString, fs};

#[test]
fn windows_hub_commands_receive_user_folders_and_system_paths_without_unrelated_values() {
    let variables = [
        ("USERPROFILE", r"C:\Users\Ada"),
        ("LOCALAPPDATA", r"C:\Users\Ada\AppData\Local"),
        ("SystemRoot", r"C:\Windows"),
        ("SECRET", "not passed"),
    ];
    let passed = environment(|name| {
        variables
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    })
    .unwrap();
    assert_eq!(passed.len(), 3);
    assert!(passed.iter().all(|(name, _)| *name != "SECRET"));
    assert_eq!(environment(|_| None), Err(LocalHubError::NoHome));
}

#[test]
fn a_windows_bundle_carries_the_executable_and_database_beside_the_app() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("database/bin")).unwrap();
    fs::write(folder.path().join("database/bin/postgres.exe"), b"fixture").unwrap();
    fs::write(folder.path().join("anima-local-hub.exe"), b"fixture").unwrap();
    let executable = folder.path().join("anima-desktop.exe");
    let found = carried::beside(&executable, false, true, &DevFolders::default()).unwrap();
    assert_eq!(found.program, folder.path().join("anima-local-hub.exe"));
    assert_eq!(found.tree, folder.path().join("database"));
    assert_eq!(
        carried::beside(&executable, false, false, &DevFolders::default()),
        Err(LocalHubError::NotCarried)
    );
}

#[test]
fn exporting_and_replacing_a_key_keeps_the_destination_private() {
    let folder = tempfile::tempdir().unwrap();
    anima_windows::files::protect(folder.path()).unwrap();
    let key = folder.path().join("vault.key");
    let destination = folder.path().join("saved.key");
    fs::write(&key, b"first fixture key").unwrap();
    key_copy::copy(&key, &destination).unwrap();
    assert!(anima_windows::files::secure(&destination, true).unwrap());
    fs::write(&key, b"replacement fixture key").unwrap();
    key_copy::copy(&key, &destination).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"replacement fixture key");
    assert!(anima_windows::files::secure(&destination, true).unwrap());
    assert_eq!(
        key_copy::copy(&key, folder.path()),
        Err(LocalHubError::Destination)
    );
    assert_eq!(
        key_copy::copy(&folder.path().join("absent.key"), &destination),
        Err(LocalHubError::NoKey)
    );
}
