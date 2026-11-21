//! The socket for the daemon that the app gives the device link (HMD-054): only one that the
//! link would accept, so that the link still starts.

use std::fs;

use super::super::kept;

#[test]
fn hmd_054_a_folder_that_others_can_reach_or_a_link_gives_no_socket() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    let open = folder.path().join("open");
    fs::create_dir(&open).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(kept::private_socket(&open), None);

    let private = folder.path().join("private");
    kept::private_socket(&private).unwrap();
    let linked = folder.path().join("linked");
    std::os::unix::fs::symlink(&private, &linked).unwrap();
    assert_eq!(kept::private_socket(&linked), None);

    let long = folder.path().join("a".repeat(100));
    assert_eq!(kept::private_socket(&long), None, "too long for a socket");
}

/// R-NEW-socketpeer: the link refuses to start with a socket that it cannot make, so the app
/// gives it none: below a folder that others can write to, or with a file that is not a
/// socket at the path.
#[test]
fn hmd_054_a_socket_that_the_link_would_refuse_is_not_given_to_it() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    let shared = folder.path().join("shared");
    fs::create_dir(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(kept::private_socket(&shared.join("daemon")), None);
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o1777)).unwrap();
    assert!(
        kept::private_socket(&shared.join("daemon")).is_some(),
        "the sticky bit"
    );

    let private = folder.path().join("private");
    let socket = kept::private_socket(&private).unwrap();
    fs::write(&socket, "not a socket").unwrap();
    assert_eq!(kept::private_socket(&private), None);
}
