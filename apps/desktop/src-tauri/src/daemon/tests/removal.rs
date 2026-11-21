//! The removal (HMD-056) with the stand-in daemon of the rig in a temporary HOME: the real
//! `~/.local/bin`, unit, `launchctl`, `systemctl` and Keychain are never touched.

use std::{fs, path::Path};

use super::rig::{NEVER, Rig, WIDE, rig};
use crate::daemon::{
    DaemonError,
    removal::{Leave, UNINSTALL_ARGS, remove_here},
};

fn remove(rig: &Rig, carried: Option<&Path>) -> Result<bool, DaemonError> {
    remove_here(&Leave {
        copy: &rig.installed(),
        carried,
        env: &rig.here().env,
        deadline: WIDE,
        stop: &NEVER,
    })
}

#[test]
fn hmd_056_the_copy_runs_uninstall_then_is_deleted() {
    let rig = rig("exit 0");
    rig.install().expect("installed");
    let before = rig.runs();

    assert_eq!(remove(&rig, Some(&rig.source)), Ok(true));

    assert!(!rig.installed().exists(), "the copy is gone");
    assert!(rig.source.exists(), "the daemon of the app stays");
    let run = rig.runs()[before.len()..].to_owned();
    assert_eq!(run, rig.run_of("uninstall", ""));
    assert_eq!(UNINSTALL_ARGS, ["uninstall"]);
}

#[test]
fn hmd_056_a_second_removal_runs_the_daemon_of_the_app_and_ends_the_same() {
    let rig = rig("exit 0");
    rig.install().expect("installed");
    remove(&rig, Some(&rig.source)).expect("removed");
    let before = rig.runs();

    assert_eq!(
        remove(&rig, Some(&rig.source)),
        Ok(false),
        "no copy was here"
    );

    let run = &rig.runs()[before.len()..];
    let program = format!("program:{}\nargs:uninstall\n", rig.source.display());
    assert!(run.starts_with(&program), "{run}");
    assert!(!rig.installed().exists());
}

#[test]
fn hmd_056_a_daemon_that_does_not_uninstall_keeps_its_copy() {
    let rig = rig("[ \"$1\" = uninstall ] && exit 1; exit 0");
    rig.install().expect("installed");

    assert_eq!(remove(&rig, Some(&rig.source)), Err(DaemonError::Uninstall));
    assert!(
        rig.installed().is_file(),
        "the unit still finds its program"
    );
}

#[test]
fn hmd_056_with_no_copy_and_no_daemon_in_the_app_nothing_runs() {
    let rig = rig("exit 0");
    assert_eq!(remove(&rig, None), Ok(false));
    assert_eq!(
        remove(&rig, Some(&rig.dir.path().join("absent"))),
        Ok(false)
    );
    assert_eq!(rig.runs(), "", "no program was started");
}

#[test]
fn hmd_056_a_copy_that_cannot_be_deleted_is_its_own_error() {
    use std::os::unix::fs::PermissionsExt;
    let rig = rig("exit 0");
    rig.install().expect("installed");
    let bin = rig.local_bin();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o555)).expect("chmod");

    let result = remove(&rig, Some(&rig.source));

    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod back");
    assert_eq!(result, Err(DaemonError::Remove));
    assert!(rig.installed().is_file());
}
