//! A join and the swap of the link's folders, with a stand-in `anima-link`.

use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

use super::super::{
    LinkError,
    device::DeviceLink,
    folder::{self, LinkFolders},
    joining, printed, replace,
    state::Phase,
};
use super::runs::{free_port, ready_then_wait, stand_in, until};

const INVITE_LINE: &str = "anima-invite.1.q83vEjRWeJCrze8SNFZ4kA.hub\n";

/// `anima-link join -state DIR` with the invite on standard input.
fn join_script() -> String {
    format!(
        r#"
[ "$1 $2" = "join -state" ] || exit 9
read -r invite
mkdir -p "$3"
case "$invite" in
  anima-invite.1.q83vEjRWeJCrze8SNFZ4kA.hub)
    echo "{}1a2b3c4d."
    echo "Start the link with: anima-link device -state $3" ;;
  *)
    echo "anima-link: devicelink: the hub did not answer; the invite is used, expired or wrong, or the hub is off" >&2
    exit 1 ;;
esac
"#,
        printed::JOINED
    )
}

/// A joined link folder with a key file and a saved port.
fn joined_folder(folder: &Path, port: u16) {
    fs::create_dir_all(folder).unwrap();
    fs::write(folder.join("key"), "").unwrap();
    folder::write_port(folder, port).unwrap();
}

#[test]
fn a_join_makes_a_new_folder_with_a_port_that_replaces_the_old_one() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), &join_script());
    let folders = LinkFolders::below(&folder.path().join("app"));
    fs::create_dir_all(&folders.device).unwrap();
    fs::write(folders.device.join("old-key"), "").unwrap();

    let joined = joining::join(&program, &folders, INVITE_LINE, &[]).unwrap();

    assert_eq!(joined.fingerprint, "1a2b3c4d");
    assert_eq!(folder::read_port(&folders.next), Some(joined.port));
    let mode = fs::metadata(folders.next.join("anima-port"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let link_mode = fs::metadata(folders.device.parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(link_mode & 0o077, 0, "{link_mode:o}");

    folders.take_next().unwrap();
    folders.remove_old().unwrap();
    assert_eq!(folder::read_port(&folders.device), Some(joined.port));
    assert!(!folders.device.join("old-key").exists());
    assert!(!folders.next.exists());
}

#[test]
fn a_join_the_hub_does_not_answer_leaves_the_old_link_alone() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), &join_script());
    let folders = LinkFolders::below(&folder.path().join("app"));
    fs::create_dir_all(&folders.device).unwrap();
    fs::write(folders.device.join("old-key"), "").unwrap();

    let error = joining::join(&program, &folders, "anima-invite.1.used.hub\n", &[]).unwrap_err();

    assert_eq!(error, LinkError::NoAnswer);
    assert!(!folders.next.exists());
    assert!(folders.device.join("old-key").exists());
}

#[test]
fn a_swap_that_fails_half_way_puts_the_old_folder_back() {
    let folder = tempfile::tempdir().unwrap();
    let folders = LinkFolders::below(folder.path());
    joined_folder(&folders.device, 41000);
    joined_folder(&folders.next, 42000);
    // The first rename (device to device.old) works; the second (next to device) fails.
    let next = folders.next.clone();
    let rename = |from: &Path, to: &Path| {
        if from == next {
            return Err(io::Error::other("the disk is full"));
        }
        fs::rename(from, to)
    };

    assert!(folders.take_next_with(rename).is_err());

    assert_eq!(folder::read_port(&folders.device), Some(41000));
    assert_eq!(folder::read_port(&folders.next), Some(42000));
}

#[test]
fn the_app_start_puts_back_a_folder_left_aside_by_a_stopped_swap() {
    let folder = tempfile::tempdir().unwrap();
    let folders = LinkFolders::below(folder.path());
    let old = folders.device.with_file_name("device.old");
    joined_folder(&old, 41000);

    folders.recover().unwrap();

    assert_eq!(folder::read_port(&folders.device), Some(41000));
    assert!(!old.exists());
    // With `device` there, an old folder is left for removal, never put over it.
    joined_folder(&old, 43000);
    folders.recover().unwrap();
    assert_eq!(folder::read_port(&folders.device), Some(41000));
}

#[test]
fn a_failed_swap_runs_the_link_before_again_on_its_own_port() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), &ready_then_wait());
    let folders = LinkFolders::below(&folder.path().join("app"));
    let (old_port, new_port) = (free_port(), free_port());
    joined_folder(&folders.device, old_port);
    joined_folder(&folders.next, new_port);
    let link = folders.device.parent().unwrap().to_path_buf();
    let device = DeviceLink::default();
    // No rename can happen in a folder its owner may not write.
    fs::set_permissions(&link, fs::Permissions::from_mode(0o500)).unwrap();

    let result = replace(&device, program, &folders, new_port);
    let state = until(&device, |state| state.phase == Phase::Up);
    device.stop();
    fs::set_permissions(&link, fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(result, Err(LinkError::Folder));
    assert!(state.serves(old_port), "{state:?}");
    assert_eq!(folder::read_port(&folders.device), Some(old_port));
}

#[test]
fn a_symbolic_link_as_the_port_file_is_not_followed() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("elsewhere"), "43123\n").unwrap();
    symlink(
        folder.path().join("elsewhere"),
        folder.path().join("anima-port"),
    )
    .unwrap();

    assert_eq!(folder::read_port(folder.path()), None);
}

#[test]
fn leaving_removes_every_folder_of_the_link() {
    let folder = tempfile::tempdir().unwrap();
    let folders = LinkFolders::below(folder.path());
    fs::create_dir_all(folders.device.join("inner")).unwrap();
    fs::create_dir_all(&folders.next).unwrap();

    folders.remove().unwrap();

    assert!(!folders.device.exists());
    assert!(!folders.next.exists());
    folders.remove().unwrap();
}
