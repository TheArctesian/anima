//! A pipe or a link where the program expects a file must not make it wait or read without end.

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::rig;
use crate::programs::{digest, version};
use crate::units::install::InstallError;
use crate::units::listen;

fn pipe(path: &Path) {
    fs::remove_file(path).ok();
    nix::unistd::mkfifo(path, nix::sys::stat::Mode::S_IRWXU).unwrap();
}

/// Runs `work` in a thread: a regression that waits fails the test and does not hang it.
fn at_once<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, result) = mpsc::channel();
    thread::spawn(move || {
        let _ = done.send(work());
    });
    result
        .recv_timeout(Duration::from_secs(10))
        .expect("the call waited for a pipe")
}

#[test]
fn a_pipe_or_a_link_at_the_lock_of_a_change_refuses_the_update_at_once() {
    for make in [pipe as fn(&Path), |path| {
        fs::remove_file(path).ok();
        std::os::unix::fs::symlink("/dev/null", path).unwrap();
    }] {
        let rig = rig();
        rig.install();
        make(&rig.folders.change_lock());
        let folders = rig.folders.clone();

        let outcome = at_once(move || super::super::super::change_lock::take(&folders).map(drop));

        assert!(
            matches!(outcome, Err(InstallError::Write { .. })),
            "{outcome:?}"
        );
    }
}

#[test]
fn a_pipe_at_the_recorded_address_or_the_version_or_the_digest_is_read_as_missing_at_once() {
    let rig = rig();
    rig.install();
    for file in [
        rig.folders.listen_file(),
        rig.folders.programs().join("version"),
        rig.folders.programs().join("digest"),
    ] {
        pipe(&file);
    }
    let folders = rig.folders.clone();
    let digest_of_source = digest::of(&rig.source).unwrap();

    let (address, installed, stored) = at_once(move || {
        (
            listen::recorded(&folders),
            version::installed(&folders),
            digest_of_source.is_stored_in(&folders),
        )
    });

    assert_eq!((address, installed, stored), (None, None, false));
}

#[test]
fn a_file_that_is_longer_than_its_limit_is_not_read() {
    let rig = rig();
    rig.install();
    fs::write(
        rig.folders.programs().join("version"),
        "1".repeat(1_000_000),
    )
    .unwrap();
    fs::write(
        rig.folders.listen_file(),
        "127.0.0.1:9000".to_owned() + &" ".repeat(1000),
    )
    .unwrap();

    assert_eq!(version::installed(&rig.folders), None);
    assert_eq!(listen::recorded(&rig.folders), None);
}

#[test]
fn an_address_with_port_0_is_not_allowed() {
    assert!(!listen::is_allowed("127.0.0.1:0".parse().unwrap()));
    assert!(!listen::is_allowed("10.0.0.1:80".parse().unwrap()));
    assert!(listen::is_allowed("127.0.0.1:8443".parse().unwrap()));
    let rig = rig();
    rig.install();
    fs::write(rig.folders.listen_file(), "127.0.0.1:0").unwrap();
    assert_eq!(listen::recorded(&rig.folders), None);
}
