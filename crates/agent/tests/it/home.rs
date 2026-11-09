//! The agent's folder and files are held to the token file's rules, since they decide where the
//! bearer goes (MET-013, MET-020).

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use anima_agent::config::Config;
use anima_agent::config::home::{Home, HomeError};
use anima_agent::config::origin::Origin;
use anima_agent::config::private_file::PrivateFileError;
use anima_agent::token_store::TokenStore;

use crate::kit::{at, token};

fn config() -> Config {
    Config {
        hub: Origin::default_hub(),
        device: uuid::Uuid::from_u128(0xd1),
        device_name: "pop-os".to_owned(),
    }
}

fn mode(path: &std::path::Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

#[test]
fn met_013_a_folder_others_can_write_is_refused_before_anything_is_read_or_written() {
    let dir = tempfile::tempdir().expect("a folder");
    mode(dir.path(), 0o700);
    let home = Home::at(dir.path().to_path_buf());
    home.write_config(&config())
        .expect("written while the folder is private");
    mode(dir.path(), 0o777);
    assert!(matches!(home.ensure(), Err(HomeError::Exposed)));
    assert!(matches!(home.read_config(), Err(HomeError::Exposed)));
    let store = TokenStore::open(&home);
    assert!(store.load().is_err());
}

#[test]
fn met_013_a_config_file_others_can_write_is_refused() {
    let dir = tempfile::tempdir().expect("a folder");
    mode(dir.path(), 0o700);
    let home = Home::at(dir.path().to_path_buf());
    home.write_config(&config()).expect("written");
    mode(&dir.path().join("agent.json"), 0o666);
    assert!(matches!(
        home.read_config(),
        Err(HomeError::File(PrivateFileError::Exposed))
    ));
}

#[test]
fn met_013_a_symlinked_home_is_refused() {
    let real = tempfile::tempdir().expect("a folder");
    let parent = tempfile::tempdir().expect("a folder");
    mode(real.path(), 0o700);
    let link = parent.path().join("agent");
    std::os::unix::fs::symlink(real.path(), &link).expect("a link");
    let home = Home::at(link);
    assert!(matches!(home.ensure(), Err(HomeError::Exposed)));
    assert!(matches!(
        home.write_config(&config()),
        Err(HomeError::Exposed)
    ));
}

#[test]
fn met_013_a_config_swapped_in_by_rename_with_loose_mode_is_not_trusted() {
    let dir = tempfile::tempdir().expect("a folder");
    mode(dir.path(), 0o700);
    let home = Home::at(dir.path().to_path_buf());
    home.write_config(&config()).expect("written");
    let evil = br#"{"hub":"https://evil.example","device":"00000000-0000-0000-0000-0000000000d1","deviceName":"x"}"#;
    let planted = dir.path().join("planted");
    fs::write(&planted, evil).expect("plant");
    fs::rename(&planted, dir.path().join("agent.json")).expect("swap");
    assert!(
        home.read_config().is_err(),
        "a default-mode file is not 0600"
    );
}

#[test]
fn met_013_a_symlink_at_the_token_path_is_replaced_and_its_target_left_alone() {
    let dir = tempfile::tempdir().expect("a folder");
    let elsewhere = tempfile::tempdir().expect("another");
    let target = elsewhere.path().join("victim");
    fs::write(&target, b"untouched").expect("victim");
    mode(dir.path(), 0o700);
    let home = Home::at(dir.path().to_path_buf());
    std::os::unix::fs::symlink(&target, home.token_path()).expect("plant");
    let credential = anima_agent::token_store::Credential {
        token: token(4),
        expires_at: at(0),
        hub: None,
    };
    TokenStore::open(&home).save(&credential).expect("saved");
    assert_eq!(fs::read(&target).expect("victim"), b"untouched");
    assert!(
        !fs::symlink_metadata(home.token_path())
            .expect("meta")
            .file_type()
            .is_symlink()
    );
}
