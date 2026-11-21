//! An update of the copy of the link program while its unit runs (HMD-054, HMD-058): the running
//! link keeps its file, so the check of the daemon at the socket finds its code on disk, until
//! the unit starts the new copy.

use std::fs;
use std::os::unix::fs::MetadataExt as _;
#[cfg(target_os = "macos")]
use std::path::Path;

use super::super::copy::{keep_running_copy, previous};

/// Set in the environment of a copy of this test program that stands in for a running link.
const STAND_IN: &str = "ANIMA_TEST_STAND_IN_LINK";

/// The stand-in link: a copy of this test program, started with [`STAND_IN`], waits here.
#[test]
fn stand_in_link_waits() {
    if std::env::var_os(STAND_IN).is_some() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}

#[test]
fn hmd_054_an_update_moves_the_running_copy_aside_once_and_keeps_its_file() {
    let folder = tempfile::tempdir().unwrap();
    let copy = folder.path().join("anima-link");
    fs::write(&copy, b"before").unwrap();
    let running = fs::metadata(&copy).unwrap().ino();

    assert!(keep_running_copy(&copy));
    let aside = previous(&copy);
    assert!(!copy.exists(), "the place is free for the new copy");
    assert_eq!(
        fs::metadata(&aside).unwrap().ino(),
        running,
        "the same file, renamed"
    );
    assert_eq!(fs::read(&aside).unwrap(), b"before");

    // A second update before the unit started again: the link may still run from the copy
    // aside, so that copy stays, and the new copy is written over in its place.
    fs::write(&copy, b"new").unwrap();
    assert!(!keep_running_copy(&copy), "the copy aside stays");
    assert_eq!(fs::read(&aside).unwrap(), b"before");
    assert_eq!(fs::read(&copy).unwrap(), b"new");
}

/// Whether `codesign --verify` finds the running process `pid` valid: its file is its code.
#[cfg(target_os = "macos")]
fn valid(pid: u32) -> bool {
    std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", &pid.to_string()])
        .env_clear()
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}

/// A stand-in link started from `program`.
#[cfg(target_os = "macos")]
fn start(program: &Path) -> std::process::Child {
    let child = std::process::Command::new(program)
        .args(["--exact", "link::tests::updates::stand_in_link_waits"])
        .env(STAND_IN, "1")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    child
}

/// A copy of this test program at `to`, signed again (ad hoc) with `identifier`, so that its
/// bytes differ from the program's own.
#[cfg(target_os = "macos")]
fn signed_copy(to: &Path, identifier: &str) {
    fs::copy(std::env::current_exe().unwrap(), to).unwrap();
    let signed = std::process::Command::new("/usr/bin/codesign")
        .args(["-f", "-s", "-", "-i", identifier])
        .arg(to)
        .env_clear()
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(signed.success());
}

/// The real `codesign` reads stand-in links (copies of this test program); no unit and no real
/// link program is touched.
#[cfg(target_os = "macos")]
#[test]
fn hmd_054_a_link_whose_copy_is_updated_stays_valid_and_its_restart_is_valid() {
    let folder = tempfile::tempdir().unwrap();
    let copy = folder.path().join("anima-link");
    signed_copy(&copy, "stand.in.before");
    let mut before = start(&copy);
    assert!(valid(before.id()));

    // The update as `place` makes it: the running copy aside, the new copy at its place.
    let new = folder.path().join("anima-link.new");
    signed_copy(&new, "stand.in.after");
    assert!(keep_running_copy(&copy));
    fs::rename(&new, &copy).unwrap();
    assert!(
        valid(before.id()),
        "the link of before still passes the check"
    );

    // The unit starts again: the new link passes too.
    before.kill().unwrap();
    before.wait().unwrap();
    let mut after = start(&copy);
    assert!(valid(after.id()));

    // Without the move aside, a copy written over a running link fails the check.
    let other = folder.path().join("anima-link.other");
    signed_copy(&other, "stand.in.other");
    fs::rename(&other, &copy).unwrap();
    assert!(!valid(after.id()));
    after.kill().unwrap();
    after.wait().unwrap();
}
