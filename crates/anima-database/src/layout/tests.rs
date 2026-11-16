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
fn linux_ignores_a_share_folder_that_is_not_absolute() {
    let folders = UserFolders {
        xdg_data_home: Some(PathBuf::from("share")),
        ..user("/home/ada")
    };

    assert_eq!(
        Layout::for_user(Platform::Linux, &folders),
        Layout::for_user(Platform::Linux, &user("/home/ada"))
    );
}

#[test]
fn linux_ignores_an_empty_runtime_folder() {
    let folders = UserFolders {
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

/// The names are what the server, the roles and the hub's `ANIMA_DATABASE_URL_FILE` rely on.
#[test]
fn a_root_with_a_space_gives_these_exact_paths() {
    let layout = Layout::new(PathBuf::from("/var/db root"), PathBuf::from("/run/pg")).unwrap();

    assert_eq!(layout.root(), Path::new("/var/db root"));
    assert_eq!(layout.cluster(), Path::new("/var/db root/cluster"));
    assert_eq!(
        layout.cluster_being_made(),
        Path::new("/var/db root/cluster.new")
    );
    assert_eq!(layout.urls_folder(), Path::new("/var/db root/urls"));
    assert_eq!(
        layout.url_file(Role::Owner),
        Path::new("/var/db root/urls/owner.url")
    );
    assert_eq!(
        layout.url_file(Role::Hub),
        Path::new("/var/db root/urls/hub.url")
    );
    assert_eq!(layout.server_log(), Path::new("/var/db root/server.log"));
    assert_eq!(layout.lock_file(), Path::new("/var/db root/lock"));
    assert_eq!(layout.socket_folder(), Path::new("/run/pg"));
    assert_eq!(layout.socket_file(), Path::new("/run/pg/.s.PGSQL.5432"));
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

#[test]
fn db_001_a_socket_folder_that_postgresql_would_read_as_a_list_is_refused() {
    for folder in ["/run/a,/tmp/b", "/run/a\"b", "/run/a "] {
        assert_eq!(
            Layout::new(PathBuf::from("/var/db"), PathBuf::from(folder)),
            Err(LayoutError::SocketFolderList),
            "{folder}"
        );
    }
    assert!(Layout::new(PathBuf::from("/var/db"), PathBuf::from("/run/a b")).is_ok());
}

#[test]
fn db_001_a_socket_folder_that_is_not_utf_8_is_refused() {
    use std::os::unix::ffi::OsStrExt as _;
    let folder = PathBuf::from(std::ffi::OsStr::from_bytes(b"/run/\xff"));

    assert_eq!(
        Layout::new(PathBuf::from("/var/db"), folder),
        Err(LayoutError::SocketFolderList)
    );
}

#[test]
fn a_folder_with_a_line_break_in_its_path_is_refused() {
    assert_eq!(
        Layout::new(PathBuf::from("/var/db\nready"), PathBuf::from("/run/pg")),
        Err(LayoutError::ControlCharacter {
            folder: "database root"
        })
    );
    assert_eq!(
        Layout::new(PathBuf::from("/var/db"), PathBuf::from("/run/pg\n")),
        Err(LayoutError::ControlCharacter {
            folder: "socket folder"
        })
    );
}
