//! The update of the installed copy with the app (HMD-055), with stand-in daemons in a temporary
//! HOME: each answers `--version` as told and records each other run with the version of the
//! installed copy and the inode of `anima-daemon.previous` at that moment, so their `stop` and
//! `install` are the fake service manager. The real `~/.local/bin`, unit, `launchctl` and
//! `systemctl` are never touched.

use std::{
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::CommandExt,
    },
    path::Path,
    process::Command,
};

use super::rig::{NEVER, Rig, WIDE, names_in, rig};
use crate::daemon::{
    DaemonError,
    update::{STOP_ARGS, Swap, Updated, previous, update_here},
    version::{Decision, decide, parse},
};

/// How a stand-in answers `--version`.
const OLD: &str = "echo 'anima-daemon 0.1.9'";
const NEW: &str = "echo 'anima-daemon 0.1.10'";
/// A daemon from before `--version`: it prints the usage and fails.
const NO_VERSION: &str = "echo 'usage: anima-daemon run' >&2; exit 2";
/// What the stand-ins record as the version of the copy.
const OLD_COPY: &str = "copy:anima-daemon 0.1.9";
const NEW_COPY: &str = "copy:anima-daemon 0.1.10";

/// A stand-in daemon named `label` that answers `--version` with `version` and records each
/// other run as `<label> <args> copy:<version of the copy> prev:<inode of the copy of before>`,
/// then runs `then`.
fn stand_in(rig: &Rig, label: &str, version: &str, then: &str) -> String {
    let copy = rig.installed();
    format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then {version}; exit 0; fi\nprintf '%s %s copy:%s prev:%s\\n' '{label}' \"$*\" \"$('{}' --version 2>/dev/null)\" \"$(ls -i '{}' 2>/dev/null | awk '{{print $1}}')\" >> '{}'\n{then}\n",
        copy.display(),
        previous(&copy).display(),
        rig.record.display()
    )
}

fn write_program(path: &Path, text: &str) {
    fs::write(path, text).expect("a stand-in");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// A rig whose installed copy (`old`) answers `installed` and whose carried daemon (`new`)
/// answers `carried` and runs `then` for each other command.
fn installed(installed: &str, carried: &str, then: &str) -> (Rig, String, String) {
    let rig = rig("exit 0");
    rig.local_bin();
    let old = stand_in(&rig, "old", installed, "exit 0");
    let new = stand_in(&rig, "new", carried, then);
    write_program(&rig.installed(), &old);
    write_program(&rig.source, &new);
    (rig, old, new)
}

fn update(rig: &Rig) -> Result<Updated, DaemonError> {
    let env = rig.here().env;
    update_here(&Swap {
        copy: &rig.installed(),
        carried: &rig.source,
        user: rig.user,
        env: &env,
        restart: WIDE,
        stop: &NEVER,
    })
}

fn copy_text(rig: &Rig) -> String {
    fs::read_to_string(rig.installed()).expect("a copy")
}

fn inode(path: &Path) -> u64 {
    fs::metadata(path).expect("a file").ino()
}

fn bin(rig: &Rig) -> Vec<String> {
    names_in(&rig.home.join(".local/bin"))
}

#[test]
fn hmd_055_a_version_is_the_name_and_three_numbers() {
    assert_eq!(parse(b"anima-daemon 0.1.12\n"), Some([0, 1, 12]));
    assert_eq!(parse(b"anima-daemon 2.0.0"), Some([2, 0, 0]));
    for refused in [
        "",
        "anima-daemon 0.1",
        "anima-daemon 0.1.2.3",
        "anima-daemon v0.1.2",
        "anima-daemon +1.2.3",
        "anima-daemon 0.1.2 extra",
        "anima-daemon 0.1.2\n\n",
        "anima-agent 0.1.2",
        "usage: anima-daemon run",
    ] {
        assert_eq!(parse(refused.as_bytes()), None, "{refused:?}");
    }
}

#[test]
fn hmd_055_versions_compare_as_numbers_and_an_unread_copy_counts_as_older() {
    assert_eq!(decide([0, 1, 10], Some([0, 1, 9])), Decision::Replace);
    assert_eq!(decide([0, 1, 9], Some([0, 1, 9])), Decision::Same);
    assert_eq!(decide([0, 1, 9], Some([0, 1, 10])), Decision::KeepNewer);
    assert_eq!(decide([0, 1, 9], Some([0, 2, 0])), Decision::KeepNewer);
    assert_eq!(decide([0, 1, 9], None), Decision::Replace);
}

#[test]
fn hmd_055_the_unit_stops_then_a_new_file_takes_the_place_then_the_unit_starts() {
    let (rig, _, new) = installed(OLD, NEW, "exit 0");
    let old_inode = inode(&rig.installed());

    assert_eq!(update(&rig), Ok(Updated::Replaced));

    assert_eq!(copy_text(&rig), new);
    assert_ne!(
        inode(&rig.installed()),
        old_inode,
        "a new file, not a write"
    );
    assert_eq!(
        rig.runs(),
        format!("new stop {OLD_COPY} prev:\nnew install {NEW_COPY} prev:{old_inode}\n"),
        "stop with the old copy in place, install with the new one while the old file waits \
         aside; no enrolment"
    );
    assert_eq!(STOP_ARGS, ["stop"]);
    assert_eq!(bin(&rig), ["anima-daemon"]);
}

#[test]
fn thr_113_the_update_never_signals_the_holder_of_the_threads() {
    let (rig, _, _) = installed(OLD, NEW, "exit 0");
    // A stand-in holder in a process group of its own, as `anima-daemon holder` runs.
    let mut holder = Command::new("/bin/sleep")
        .arg("60")
        .process_group(0)
        .spawn()
        .expect("a holder");

    assert_eq!(update(&rig), Ok(Updated::Replaced));

    let alive = holder.try_wait().expect("a state").is_none();
    holder.kill().expect("killed");
    holder.wait().expect("reaped");
    assert!(alive, "the holder still runs after the update");
}

#[test]
fn hmd_055_the_same_version_changes_nothing() {
    let (rig, old, _) = installed(NEW, NEW, "exit 0");

    assert_eq!(update(&rig), Ok(Updated::Unchanged));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(rig.runs(), "", "the unit is not stopped or started");
}

#[test]
fn hmd_055_an_older_carried_daemon_never_goes_over_a_newer_copy() {
    let (rig, newer, _) = installed(NEW, OLD, "exit 0");

    assert_eq!(update(&rig), Ok(Updated::KeptNewer));

    assert_eq!(copy_text(&rig), newer);
    assert_eq!(rig.runs(), "");
}

#[test]
fn hmd_055_a_copy_that_prints_no_version_is_replaced() {
    let (rig, _, new) = installed(NO_VERSION, NEW, "exit 0");

    assert_eq!(update(&rig), Ok(Updated::Replaced));

    assert_eq!(copy_text(&rig), new);
    assert_eq!(rig.runs().matches(" install ").count(), 1);
}

#[test]
fn hmd_055_a_carried_daemon_that_prints_no_version_changes_nothing() {
    let (rig, old, _) = installed(OLD, NO_VERSION, "exit 0");

    assert_eq!(update(&rig), Ok(Updated::CarriedUnknown));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(rig.runs(), "");
}

#[test]
fn hmd_055_with_no_copy_nothing_is_installed() {
    let rig = rig("exit 0");
    write_program(&rig.source, &stand_in(&rig, "new", NEW, "exit 0"));

    assert_eq!(update(&rig), Ok(Updated::NoCopy));

    assert!(
        !rig.installed().exists(),
        "the install of HMD-053 does that"
    );
    assert_eq!(rig.runs(), "");
}

#[test]
fn hmd_055_a_stop_that_fails_changes_nothing() {
    let (rig, old, _) = installed(OLD, NEW, "[ \"$1\" = stop ] && exit 1; exit 0");

    assert_eq!(update(&rig), Err(DaemonError::Failed));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(rig.runs(), format!("new stop {OLD_COPY} prev:\n"));
}

#[test]
fn hmd_055_a_copy_that_fails_after_the_stop_puts_the_copy_of_before_back_and_starts_it() {
    // The carried daemon becomes unreadable at its stop: the copy then starts, and fails.
    let (rig, old, _) = installed(OLD, NEW, "[ \"$1\" = stop ] && chmod 000 \"$0\"; exit 0");
    let old_inode = inode(&rig.installed());

    assert_eq!(update(&rig), Err(DaemonError::NotBundled));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(inode(&rig.installed()), old_inode, "the same file is back");
    assert_eq!(bin(&rig), ["anima-daemon"]);
    assert_eq!(
        rig.runs(),
        format!("new stop {OLD_COPY} prev:\nold install {OLD_COPY} prev:\n")
    );
}

#[test]
fn hmd_055_a_new_copy_whose_unit_does_not_start_gives_way_to_the_copy_of_before() {
    let (rig, old, _) = installed(OLD, NEW, "[ \"$1\" = install ] && exit 1; exit 0");
    let old_inode = inode(&rig.installed());

    assert_eq!(update(&rig), Err(DaemonError::Install));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(
        rig.runs(),
        format!(
            "new stop {OLD_COPY} prev:\nnew install {NEW_COPY} prev:{old_inode}\nold install {OLD_COPY} prev:\n"
        ),
        "the new copy, then the copy of before"
    );
    assert_eq!(bin(&rig), ["anima-daemon"]);
}

#[test]
fn hmd_055_an_update_cut_between_its_renames_puts_the_copy_of_before_back_first() {
    let (rig, old, _) = installed(NEW, NEW, "exit 0");
    fs::rename(rig.installed(), previous(&rig.installed())).expect("moved aside");

    assert_eq!(update(&rig), Ok(Updated::Unchanged));

    assert_eq!(copy_text(&rig), old);
    assert_eq!(bin(&rig), ["anima-daemon"]);
}

#[test]
fn hmd_055_an_update_cut_before_its_start_starts_the_unit_and_forgets_the_copy_of_before() {
    let (rig, _, new) = installed(NEW, NEW, "exit 0");
    write_program(
        &previous(&rig.installed()),
        &stand_in(&rig, "old", OLD, "exit 0"),
    );
    write_program(&rig.installed(), &new);

    assert_eq!(update(&rig), Ok(Updated::Unchanged));

    assert_eq!(copy_text(&rig), new);
    assert_eq!(rig.runs().matches(" install ").count(), 1);
    assert_eq!(bin(&rig), ["anima-daemon"]);
}
