//! Processes that leave the step's process group with `setsid`, as `pg_ctl` does, are ended too
//! (S-1). Linux only: there is no subreaper on macOS.

use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use super::child_process::{alive, collecting, command};
use crate::builder::{
    child::Running,
    error::BuilderError,
    reaper::{Process, descendants, marked, process_of},
};

async fn gone_within(pid: i32, limit: Duration) -> bool {
    let by = Instant::now() + limit;
    while alive(pid) && Instant::now() < by {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    !alive(pid)
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn s_1_a_process_that_left_the_group_with_setsid_does_not_outlive_the_step() {
    let folder = tempfile::tempdir().unwrap();
    let marker = folder.path().join("marker");
    let running = Running::default();
    let (sink, _) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    // The reviewer's case: an escaped process that writes after the step has ended.
    let script = format!(
        "setsid sh -c 'sleep 2; echo late > {}' & sleep 0.5",
        marker.display()
    );
    claim
        .run(&command(&["-c", &script]), Path::new("/bin/sh"), Vec::new())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert!(!marker.exists(), "an escaped process outlived the step");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn s_1_a_server_started_like_pg_ctl_is_ended_with_the_step() {
    let running = Running::default();
    let (sink, _) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    // pg_ctl starts the server in a session of its own and returns at once.
    let script = "setsid sleep 60 </dev/null >/dev/null 2>&1 & echo \"$!\"";
    let kept = claim
        .run(&command(&["-c", script]), Path::new("/bin/sh"), Vec::new())
        .await
        .unwrap();
    let server: i32 = kept[0].parse().unwrap();
    assert!(
        gone_within(server, Duration::from_secs(5)).await,
        "the server outlived the step"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s_1_cancel_ends_a_process_that_left_the_group() {
    let running = Arc::new(Running::default());
    let (sink, seen) = collecting();
    let script = command(&[
        "-c",
        "setsid sleep 60 </dev/null >/dev/null 2>&1 & echo \"$!\"; sleep 60",
    ]);
    let step = {
        let running = Arc::clone(&running);
        tokio::spawn(async move {
            let claim = running.claim(Arc::clone(&sink)).unwrap();
            claim.run(&script, Path::new("/bin/sh"), Vec::new()).await
        })
    };
    let started = Instant::now();
    let escaped = loop {
        if let Some(line) = seen.lock().unwrap().first() {
            break line.parse::<i32>().unwrap();
        }
        assert!(started.elapsed() < Duration::from_secs(10), "no output");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    running.cancel();
    assert_eq!(step.await.unwrap(), Err(BuilderError::Cancelled));
    assert!(
        gone_within(escaped, Duration::from_secs(5)).await,
        "it outlived the cancel"
    );
}

#[test]
fn s_1_stat_is_read_after_the_last_parenthesis() {
    let stat = "4242 (evil) Z 1 2 3) S 77 4242 4300 0 -1";
    assert_eq!(
        process_of(4242, stat),
        Some(Process {
            pid: 4242,
            parent: 77,
            group: 4242,
            session: 4300,
            zombie: false
        })
    );
    let zombie = process_of(9, "9 (sh) Z 77 9 9 0").unwrap();
    assert!(zombie.zombie);
    assert_eq!(process_of(9, "9 (sh"), None);
    assert_eq!(process_of(9, "9 (sh) S x 1"), None);
}

#[test]
fn s_1_descendants_are_found_through_every_generation_and_only_below_the_root() {
    let process = |pid, parent| Process {
        pid,
        parent,
        group: pid,
        session: 1,
        zombie: false,
    };
    let all = [
        process(10, 1),
        process(11, 10),
        process(12, 11),
        process(13, 1),
        process(14, 12),
    ];
    let found: Vec<i32> = descendants(10, &all).iter().map(|p| p.pid).collect();
    let mut sorted = found.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, [11, 12, 14]);
}

#[test]
fn s_1_the_marker_matches_only_its_whole_entry() {
    let environ = b"HOME=/home/op\0ANIMA_BUILD_STEP=42-7\0PATH=/usr/bin\0";
    assert!(marked(environ, "42-7"));
    assert!(!marked(environ, "42-"));
    assert!(!marked(environ, "42-70"));
    assert!(!marked(b"ANIMA_BUILD_STEP=42-7x\0", "42-7"));
}
