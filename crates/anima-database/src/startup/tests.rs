use std::fs;

use super::{BackupExclusion, Running, StartError, start, status};
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
fn a_new_cluster_gets_new_logins_also_when_old_url_files_are_there() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    start(&local_tree(), &layout, SETTINGS, LIMITS)
        .unwrap()
        .stop(LIMITS)
        .unwrap();
    let old = hub_password(&layout);
    fs::remove_dir_all(layout.cluster()).unwrap();

    let _running = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert_ne!(hub_password(&layout), old);
}

#[test]
fn a_missing_owner_file_alone_also_gets_the_logins_made_again() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    start(&local_tree(), &layout, SETTINGS, LIMITS)
        .unwrap()
        .stop(LIMITS)
        .unwrap();
    fs::remove_file(layout.url_file(Role::Owner)).unwrap();

    let _running = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    assert!(urls::password(&layout, Role::Owner).unwrap().is_some());
}

#[test]
fn a_cluster_of_another_major_version_is_refused_before_a_server_starts() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    start(&local_tree(), &layout, SETTINGS, LIMITS)
        .unwrap()
        .stop(LIMITS)
        .unwrap();
    let version_file = layout.cluster().join("PG_VERSION");

    fs::write(&version_file, "18\n").unwrap();
    let newer = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap_err();
    fs::write(&version_file, "16\n").unwrap();
    let older = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap_err();

    assert!(
        matches!(
            newer,
            StartError::ClusterNewer {
                cluster: 18,
                tree: 17
            }
        ),
        "{newer}"
    );
    assert!(
        matches!(
            older,
            StartError::ClusterOlder {
                cluster: 16,
                tree: 17
            }
        ),
        "{older}"
    );
    assert!(newer.is_version_refusal() && older.is_version_refusal());
    assert!(!layout.server_log().exists() || !layout.cluster().join("postmaster.pid").exists());
}

#[test]
fn a_new_cluster_on_macos_is_excluded_from_time_machine() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());

    let first = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();

    if cfg!(target_os = "macos") {
        assert_eq!(first.backup_exclusion, BackupExclusion::Set);
        let asked = std::process::Command::new("/usr/bin/tmutil")
            .arg("isexcluded")
            .arg(layout.cluster())
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&asked.stdout).contains("[Excluded]"),
            "{asked:?}"
        );
    } else {
        assert_eq!(first.backup_exclusion, BackupExclusion::NotDue);
    }
    first.stop(LIMITS).unwrap();
    let second = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();
    assert_eq!(second.backup_exclusion, BackupExclusion::NotDue);
}

#[test]
fn status_looks_without_the_lock_and_tells_if_a_server_accepts_connections() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let nothing = status(&local_tree(), &layout).unwrap();
    assert_eq!((nothing.tree, nothing.cluster), (17, None));
    assert!(!nothing.server_accepts_connections);

    let running = start(&local_tree(), &layout, SETTINGS, LIMITS).unwrap();
    let up = status(&local_tree(), &layout).unwrap();
    assert_eq!(up.cluster, Some(17));
    assert!(up.server_accepts_connections);

    running.stop(LIMITS).unwrap();
    assert!(
        !status(&local_tree(), &layout)
            .unwrap()
            .server_accepts_connections
    );
}
