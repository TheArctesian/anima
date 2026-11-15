use std::path::{Path, PathBuf};

use super::{Layout, LayoutError, Platform, Role, UserFolders};

fn user(home: &str) -> UserFolders {
    UserFolders {
        home: PathBuf::from(home),
        xdg_data_home: None,
        xdg_runtime_dir: None,
    }
}

/// A socket folder for which the socket file's whole path has exactly `bytes` bytes.
fn socket_folder_for(bytes: usize) -> PathBuf {
    let file = "/.s.PGSQL.5432".len();
    PathBuf::from(format!("/{}", "s".repeat(bytes - file - 1)))
}

#[test]
fn macos_keeps_the_root_and_the_socket_in_application_support() {
    let layout = Layout::for_user(Platform::MacOs, &user("/Users/ada")).unwrap();

    assert_eq!(
        layout.root(),
        Path::new("/Users/ada/Library/Application Support/ing.anima.desktop/hub/database")
    );
    assert_eq!(
        layout.socket_file(),
        Path::new("/Users/ada/Library/Application Support/ing.anima.desktop/pg/.s.PGSQL.5432")
    );
}

#[test]
fn linux_uses_the_default_share_folder_when_xdg_gives_none() {
    let layout = Layout::for_user(Platform::Linux, &user("/home/ada")).unwrap();

    assert_eq!(
        layout.root(),
        Path::new("/home/ada/.local/share/anima/hub/database")
    );
    assert_eq!(
        layout.socket_folder(),
        Path::new("/home/ada/.local/share/anima/pg")
    );
}

#[test]
fn linux_follows_the_xdg_folders() {
    let folders = UserFolders {
        xdg_data_home: Some(PathBuf::from("/srv/ada/share")),
        xdg_runtime_dir: Some(PathBuf::from("/run/user/1000")),
        ..user("/home/ada")
    };

    let layout = Layout::for_user(Platform::Linux, &folders).unwrap();

    assert_eq!(
        layout.root(),
        Path::new("/srv/ada/share/anima/hub/database")
    );
    assert_eq!(
        layout.socket_file(),
        Path::new("/run/user/1000/anima/.s.PGSQL.5432")
    );
}

#[test]
fn linux_ignores_an_xdg_folder_that_is_not_absolute() {
    let folders = UserFolders {
        xdg_data_home: Some(PathBuf::from("share")),
        xdg_runtime_dir: Some(PathBuf::new()),
        ..user("/home/ada")
    };

    assert_eq!(
        Layout::for_user(Platform::Linux, &folders),
        Layout::for_user(Platform::Linux, &user("/home/ada"))
    );
}

#[test]
fn macos_ignores_the_xdg_folders() {
    let folders = UserFolders {
        xdg_data_home: Some(PathBuf::from("/srv/share")),
        xdg_runtime_dir: Some(PathBuf::from("/run/user/1000")),
        ..user("/Users/ada")
    };

    assert_eq!(
        Layout::for_user(Platform::MacOs, &folders),
        Layout::for_user(Platform::MacOs, &user("/Users/ada"))
    );
}

#[test]
fn every_file_but_the_socket_is_below_the_root() {
    let layout = Layout::new(PathBuf::from("/var/db root"), PathBuf::from("/run/pg")).unwrap();

    let files = [
        layout.cluster(),
        layout.url_file(Role::Owner),
        layout.url_file(Role::Hub),
        layout.server_log(),
        layout.lock_file(),
    ];

    for file in &files {
        assert!(file.starts_with("/var/db root"), "{}", file.display());
    }
    assert!(!layout.socket_file().starts_with(layout.root()));
}

#[test]
fn each_role_and_each_purpose_has_its_own_file() {
    let layout = Layout::new(PathBuf::from("/var/db"), PathBuf::from("/run/pg")).unwrap();

    let mut files = vec![
        layout.cluster(),
        layout.url_file(Role::Owner),
        layout.url_file(Role::Hub),
        layout.server_log(),
        layout.lock_file(),
        layout.socket_file(),
    ];
    let count = files.len();
    files.sort();
    files.dedup();

    assert_eq!(files.len(), count);
}

#[test]
fn the_url_files_are_alone_in_their_folder() {
    let layout = Layout::new(PathBuf::from("/var/db"), PathBuf::from("/run/pg")).unwrap();

    let folder = layout.url_file(Role::Hub).parent().map(Path::to_path_buf);

    assert_eq!(
        layout.url_file(Role::Owner).parent().map(Path::to_path_buf),
        folder
    );
    assert!(!layout.cluster().starts_with(folder.as_deref().unwrap()));
    assert!(!layout.server_log().starts_with(folder.as_deref().unwrap()));
}

#[test]
fn a_socket_path_of_103_bytes_is_accepted() {
    let layout = Layout::new(PathBuf::from("/var/db"), socket_folder_for(103)).unwrap();

    assert_eq!(layout.socket_file().as_os_str().len(), 103);
}

#[test]
fn a_socket_path_of_104_bytes_is_refused() {
    assert_eq!(
        Layout::new(PathBuf::from("/var/db"), socket_folder_for(104)),
        Err(LayoutError::SocketPathTooLong {
            length: 104,
            limit: 103
        })
    );
}

#[test]
fn the_socket_limit_counts_bytes_not_characters() {
    // 52 two-byte characters: 53 characters with the slash, 105 bytes, before the file name.
    let folder = PathBuf::from(format!("/{}", "é".repeat(52)));

    assert!(matches!(
        Layout::new(PathBuf::from("/var/db"), folder),
        Err(LayoutError::SocketPathTooLong { .. })
    ));
}

#[test]
fn a_macos_home_of_40_bytes_fits_and_41_does_not() {
    let fits = format!("/Users/{}", "a".repeat(33));
    let too_long = format!("/Users/{}", "a".repeat(34));

    assert!(Layout::for_user(Platform::MacOs, &user(&fits)).is_ok());
    assert!(matches!(
        Layout::for_user(Platform::MacOs, &user(&too_long)),
        Err(LayoutError::SocketPathTooLong { length: 104, .. })
    ));
}

#[test]
fn a_long_root_is_accepted_when_the_socket_folder_is_short() {
    let root = PathBuf::from(format!("/{}", "r".repeat(300)));

    assert!(Layout::new(root, PathBuf::from("/run/pg")).is_ok());
}

#[test]
fn a_relative_folder_is_refused_and_named() {
    assert_eq!(
        Layout::new(PathBuf::from("db"), PathBuf::from("/run/pg")),
        Err(LayoutError::NotAbsolute {
            folder: "database root"
        })
    );
    assert_eq!(
        Layout::new(PathBuf::from("/var/db"), PathBuf::from("pg")),
        Err(LayoutError::NotAbsolute {
            folder: "socket folder"
        })
    );
    assert_eq!(
        Layout::for_user(Platform::Linux, &user("ada")),
        Err(LayoutError::NotAbsolute {
            folder: "home folder"
        })
    );
}
