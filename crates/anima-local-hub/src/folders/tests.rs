use std::path::PathBuf;

use anima_database::{Platform, UserFolders};

use super::Folders;

fn user(home: &str) -> UserFolders {
    UserFolders {
        home: PathBuf::from(home),
        xdg_data_home: None,
        xdg_runtime_dir: Some(PathBuf::from("/run/user/1000")),
    }
}

#[test]
fn the_database_of_s3_is_one_folder_below_the_hub_root() {
    for platform in [Platform::MacOs, Platform::Linux] {
        let folders = Folders::for_user(platform, &user("/home/ada")).unwrap();

        assert_eq!(folders.database().root(), folders.root().join("database"));
    }
}

#[test]
fn a_mac_keeps_the_hub_in_the_applications_own_folder() {
    let folders = Folders::for_user(Platform::MacOs, &user("/Users/ada")).unwrap();

    assert_eq!(
        folders.root(),
        PathBuf::from("/Users/ada/Library/Application Support/ing.anima.desktop/hub")
    );
}

#[test]
fn linux_keeps_the_hub_in_the_users_share_folder() {
    let folders = Folders::for_user(Platform::Linux, &user("/home/ada")).unwrap();

    assert_eq!(
        folders.root(),
        PathBuf::from("/home/ada/.local/share/anima/hub")
    );
    assert_eq!(
        folders.database().socket_folder(),
        PathBuf::from("/run/user/1000/anima")
    );
}

#[test]
fn a_given_root_holds_everything_but_the_socket() {
    let folders = Folders::new(PathBuf::from("/srv/trial"), PathBuf::from("/srv/s")).unwrap();

    for path in [
        folders.programs(),
        folders.programs_being_copied(),
        folders.programs_replaced(),
        folders.vault_key(),
        folders.data_keys(),
        folders.files(),
        folders.logs(),
        folders.lock_file(),
        folders.state_file(),
        folders.link(),
        folders.link_control(),
        folders.database().root().to_path_buf(),
    ] {
        assert!(path.starts_with("/srv/trial"), "{}", path.display());
    }
    // S5: the hub's socket for the link is in the short folder of the database's socket.
    assert_eq!(folders.link_socket(), PathBuf::from("/srv/s/link.sock"));
}

#[test]
fn a_macs_link_sockets_fit_the_socket_path_limit_for_a_usual_home() {
    let home = "/Users/a-longer-user-name";
    let folders = Folders::for_user(Platform::MacOs, &user(home)).unwrap();

    assert_eq!(
        folders.link_socket(),
        PathBuf::from(format!(
            "{home}/Library/Application Support/ing.anima.desktop/pg/link.sock"
        ))
    );
    assert!(folders.link_control().as_os_str().len() <= crate::members::SOCKET_PATH_LIMIT);
}

#[test]
fn a_root_that_the_database_refuses_is_refused() {
    assert!(Folders::new(PathBuf::from("relative"), PathBuf::from("/srv/s")).is_err());
}
