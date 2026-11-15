use std::fs;

use super::{Running, StartError, start};
use crate::scratch::{LIMITS, SETTINGS, local_tree};
use crate::{Layout, Role, ServerError, Stopped, urls};

fn layout_in(folder: &std::path::Path) -> Layout {
    Layout::new(folder.join("db root"), folder.join("s")).unwrap()
}

fn server_process(layout: &Layout) -> u32 {
    let pid_file = fs::read_to_string(layout.cluster().join("postmaster.pid")).unwrap();
    pid_file.lines().next().unwrap().parse().unwrap()
}

fn hub_password(layout: &Layout) -> String {
    urls::password(layout, Role::Hub)
        .unwrap()
        .unwrap()
        .expose()
        .to_owned()
}

#[test]
fn the_first_start_makes_everything_and_the_second_keeps_it() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());

    let first = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();
    assert!(!first.recovered);
    let password = hub_password(&layout);
    let made = fs::metadata(layout.cluster().join("PG_VERSION"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(first.stop(LIMITS).unwrap(), Stopped::Clean);

    let second = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert!(!second.recovered);
    assert_eq!(hub_password(&layout), password, "the logins are made once");
    assert_eq!(
        fs::metadata(layout.cluster().join("PG_VERSION"))
            .unwrap()
            .modified()
            .unwrap(),
        made,
        "the cluster is made once"
    );
}

#[test]
fn a_second_copy_is_refused_and_the_first_keeps_running() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let mut first = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    let second = start(&local_tree(), &layout, SETTINGS, LIMITS);

    assert!(
        matches!(second, Err(StartError::Server(ServerError::AnotherCopy))),
        "{second:?}"
    );
    assert!(first.ended().unwrap().is_none());
}

#[test]
fn a_start_after_a_killed_copy_takes_over_from_the_server_it_left() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let killed = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();
    let left_server = server_process(&layout);
    // The copy dies with no chance to stop its server: the lock ends, the server stays.
    let Running {
        server,
        _lock: lock,
        ..
    } = killed;
    std::mem::forget(server);
    drop(lock);

    let next = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert_ne!(server_process(&layout), left_server);
    assert!(
        !next.recovered,
        "the server that was left got a fast shutdown"
    );
    assert_eq!(next.stop(LIMITS).unwrap(), Stopped::Clean);
}

#[test]
fn a_start_after_a_killed_server_says_that_it_recovered() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let mut running = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();
    let server = nix::unistd::Pid::from_raw(i32::try_from(server_process(&layout)).unwrap());
    nix::sys::signal::kill(server, nix::sys::signal::Signal::SIGKILL).unwrap();
    while running.ended().unwrap().is_none() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(running);

    let next = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert!(next.recovered);
}

#[test]
fn a_crash_between_the_roles_and_their_files_is_mended_at_the_next_start() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    start(&local_tree(), &layout, SETTINGS, LIMITS)
        .unwrap()
        .stop(LIMITS)
        .unwrap();
    fs::remove_file(layout.url_file(Role::Hub)).unwrap();

    let _running = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert_eq!(hub_password(&layout).len(), 48);
}

#[test]
fn a_cluster_without_its_superuser_file_is_not_given_new_logins() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    start(&local_tree(), &layout, SETTINGS, LIMITS)
        .unwrap()
        .stop(LIMITS)
        .unwrap();
    fs::remove_file(layout.url_file(Role::Hub)).unwrap();
    fs::remove_file(layout.url_file(Role::Superuser)).unwrap();

    let outcome = start(&local_tree(), &layout, SETTINGS, LIMITS);

    assert!(
        matches!(outcome, Err(StartError::SuperuserFileMissing)),
        "{outcome:?}"
    );
    assert!(
        !layout.cluster().join("postmaster.pid").exists(),
        "no server is left"
    );
}
