//! The tree of a server (R-NEW-containment): the identity check before a signal, the walk of
//! the descendants on macOS, and the sweep of the records at the start of the daemon.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anima_daemon::model::tree::walk::{Row, descendants, read_table};
use anima_daemon::model::tree::{Identity, RECORD, Record, identity, signal, sweep};
use rustix::process::Signal;

fn sleeper() -> Child {
    Command::new("/bin/sleep")
        .arg("30")
        .stdin(Stdio::null())
        .spawn()
        .expect("a sleeper")
}

fn alive(child: &mut Child) -> bool {
    std::thread::sleep(Duration::from_millis(200));
    child.try_wait().expect("a status").is_none()
}

fn pid_of(child: &Child) -> i32 {
    i32::try_from(child.id()).expect("a pid")
}

#[test]
fn a_reused_process_id_is_never_signalled() {
    let mut child = sleeper();
    let wrong = Identity {
        pid: pid_of(&child),
        start: "a different start".to_owned(),
    };
    assert!(!signal(&wrong, Signal::KILL));
    assert!(alive(&mut child));
    let right = identity(pid_of(&child)).expect("an identity");
    assert!(signal(&right, Signal::KILL));
    assert!(!alive(&mut child));
}

#[test]
fn the_sweep_ends_the_processes_of_each_record_whose_identity_holds() {
    let state = tempfile::tempdir().expect("a folder");
    let mut ended = sleeper();
    let mut kept = sleeper();
    let record = Record {
        known: vec![
            identity(pid_of(&ended)).expect("an identity"),
            Identity {
                pid: pid_of(&kept),
                start: "an earlier process".to_owned(),
            },
        ],
        cgroup: None,
    };
    let folder = state.path().join("0192f6aa-0000-7000-8000-000000000042");
    std::fs::create_dir_all(&folder).expect("made");
    record.save(&folder.join(RECORD));
    assert_eq!(sweep(state.path()).expect("swept"), 1);
    assert!(!alive(&mut ended));
    assert!(
        alive(&mut kept),
        "a process whose identity changed was signalled"
    );
    assert!(!folder.join(RECORD).exists());
    kept.kill().expect("killed");
    let _ = kept.wait();
}

#[test]
fn the_walk_keeps_a_child_that_was_seen_after_its_parent_left() {
    let row = |pid, parent, start: &str| Row {
        pid,
        parent,
        start: start.to_owned(),
    };
    let table = [
        row(1, 0, "boot"),
        row(10, 1, "s"),
        row(11, 10, "c"),
        row(12, 1, "g"),
        row(13, 12, "h"),
        row(20, 1, "o"),
    ];
    let leader = Identity {
        pid: 10,
        start: "s".to_owned(),
    };
    let found = descendants(&table, &[leader]);
    assert_eq!(found.iter().map(|id| id.pid).collect::<Vec<_>>(), [10, 11]);
    // The leader left; its grandchild 13 went to process 1, and an earlier poll saw 12.
    let later = [
        row(1, 0, "boot"),
        row(12, 1, "g"),
        row(13, 12, "h"),
        row(20, 1, "o"),
    ];
    let roots = [
        Identity {
            pid: 10,
            start: "s".to_owned(),
        },
        Identity {
            pid: 12,
            start: "g".to_owned(),
        },
    ];
    let mut found: Vec<i32> = descendants(&later, &roots)
        .iter()
        .map(|id| id.pid)
        .collect();
    found.sort_unstable();
    assert_eq!(found, [12, 13]);
    let reused = [Identity {
        pid: 12,
        start: "another".to_owned(),
    }];
    assert_eq!(descendants(&later, &reused), []);
}

#[test]
fn the_process_table_reads() {
    let rows =
        read_table("  1     0 Thu Nov 20 10:00:00 2026\n 42     1 Thu Nov 20 10:01:02 2026\nbad\n");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1],
        Row {
            pid: 42,
            parent: 1,
            start: "Thu Nov 20 10:01:02 2026".to_owned()
        }
    );
}

#[cfg(target_os = "macos")]
#[test]
fn the_walk_finds_a_live_child_on_this_mac() {
    let mut shell = Command::new("/bin/sh")
        .args(["-c", "/bin/sleep 30 & wait"])
        .env_clear()
        .stdin(Stdio::null())
        .spawn()
        .expect("a shell");
    std::thread::sleep(Duration::from_millis(300));
    let table = anima_daemon::model::tree::walk::table().expect("a table");
    let leader = identity(pid_of(&shell)).expect("an identity");
    let found = descendants(&table, std::slice::from_ref(&leader));
    assert_eq!(found.len(), 2, "{found:?}");
    for member in &found {
        assert!(signal(member, Signal::KILL));
    }
    let _ = shell.wait();
}

#[tokio::test]
async fn an_empty_tree_of_a_running_server_does_not_read_as_a_safe_table() {
    assert!(anima_daemon::model::sockets::table(&[]).await.is_err());
}

fn exiting(root: &std::path::Path, script: &str) -> anima_daemon::model::server::Launch {
    let place = root.join("servers/one");
    anima_daemon::model::server::Launch {
        program: "/bin/sh".into(),
        arguments: vec!["-c".to_owned(), script.to_owned()],
        environment: Vec::new(),
        state: root.to_path_buf(),
        temporary: place.join("tmp"),
        ring: place.join("ring"),
        key: zeroize::Zeroizing::new(String::new()),
        place,
        server: uuid::Uuid::nil(),
        launcher: env!("CARGO_BIN_EXE_anima-daemon").into(),
    }
}

#[tokio::test]
async fn an_exit_code_of_a_launcher_step_counts_only_with_the_failure_file_of_the_launcher() {
    use daemon_core::model_work::cause::Containment;
    if !super::kit::contained() {
        return;
    }
    let root = tempfile::tempdir().expect("a folder");
    let mut runtime =
        anima_daemon::model::server::spawn(&exiting(root.path(), "sleep 0.5; exit 123"))
            .expect("started");
    let status = runtime.child.wait().await.expect("ended");
    assert_eq!(
        runtime.launch_failure(status),
        None,
        "the runtime's own 123"
    );
    let written = "sleep 0.5; printf 123 > ../launch-failure; exit 123";
    let mut launcher =
        anima_daemon::model::server::spawn(&exiting(root.path(), written)).expect("started");
    let status = launcher.child.wait().await.expect("ended");
    assert_eq!(
        launcher.launch_failure(status),
        Some(Containment::UsernsDenied)
    );
}

#[test]
fn the_cgroup_mount_must_delegate_to_namespaces() {
    use anima_daemon::model::tree::mounts::cgroup_has_nsdelegate;
    let line = |options: &str| {
        format!(
            "30 24 0:26 / /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime shared:4 - cgroup2 cgroup2 {options}\n"
        )
    };
    assert!(cgroup_has_nsdelegate(&line(
        "rw,nsdelegate,memory_recursiveprot"
    )));
    assert!(!cgroup_has_nsdelegate(&line("rw,memory_recursiveprot")));
    assert!(!cgroup_has_nsdelegate(
        "31 24 0:27 / /mnt/other rw shared:5 - cgroup2 cgroup2 rw,nsdelegate\n"
    ));
    assert!(!cgroup_has_nsdelegate(""));
}
