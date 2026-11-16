//! The ownership check of the control socket: the rule of `link/statedir` (FRN-009).

use std::{
    fs,
    os::unix::fs::{PermissionsExt as _, symlink},
    path::Path,
};

use tokio::net::UnixListener;

use super::{fake, uid};
use crate::fleet::link::{Error, owned};

fn exposed(path: &Path, uid: u32) -> bool {
    matches!(owned::check(path, uid), Err(Error::Exposed(_)))
}

#[tokio::test]
async fn frn_009_the_socket_and_its_folder_must_be_the_hub_users_and_owner_only() {
    let (folder, path) = fake(|_| Vec::new());
    assert!(owned::check(&path, uid()).is_ok());
    assert!(exposed(&path, uid() + 1), "another user's socket");

    fs::set_permissions(&path, fs::Permissions::from_mode(0o660)).unwrap();
    assert!(exposed(&path, uid()), "a socket the group can reach");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(exposed(&path, uid()), "a folder the group can reach");
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();

    let regular = folder.path().join("file");
    fs::write(&regular, "").unwrap();
    fs::set_permissions(&regular, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(exposed(&regular, uid()), "not a socket");

    let linked = folder.path().join("linked.sock");
    symlink(&path, &linked).unwrap();
    assert!(exposed(&linked, uid()), "the socket itself a symbolic link");
}

#[tokio::test]
async fn frn_009_a_folder_above_that_others_can_write_is_refused() {
    let (outer, _) = fake(|_| Vec::new());
    let open = outer.path().join("open");
    fs::create_dir(&open).unwrap();
    let inner = open.join("link");
    fs::create_dir(&inner).unwrap();
    fs::set_permissions(&inner, fs::Permissions::from_mode(0o700)).unwrap();
    let path = inner.join("control.sock");
    let _listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(owned::check(&path, uid()).is_ok());
    fs::set_permissions(&open, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(
        exposed(&path, uid()),
        "a folder above that others can write"
    );
    fs::set_permissions(&open, fs::Permissions::from_mode(0o1777)).unwrap();
    assert!(
        owned::check(&path, uid()).is_ok(),
        "the sticky bit stops a replacement"
    );
}

/// An ACL entry admits another user whatever the mode says (macOS, `chmod +a`).
#[cfg(target_os = "macos")]
#[tokio::test]
async fn frn_009_a_socket_or_folder_with_an_access_control_list_is_refused() {
    let (folder, path) = fake(|_| Vec::new());
    assert!(owned::check(&path, uid()).is_ok());
    for target in [folder.path(), path.as_path()] {
        let entry = "everyone allow read";
        let added = std::process::Command::new("/bin/chmod")
            .args(["+a", entry])
            .arg(target)
            .status()
            .unwrap();
        assert!(added.success());
        assert!(exposed(&path, uid()), "an ACL on {}", target.display());
        let removed = std::process::Command::new("/bin/chmod")
            .arg("-N")
            .arg(target)
            .status()
            .unwrap();
        assert!(removed.success());
        assert!(owned::check(&path, uid()).is_ok());
    }
}
