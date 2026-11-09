//! Real processes: `/usr/bin/env` and `/bin/sh`, which every builder computer has.

use std::{
    ffi::OsString,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use super::{COMMIT, parent_with_secrets};
use crate::builder::{
    child::{Running, Sink},
    environment::Environment,
    error::BuilderError,
    plan::{Command, Program},
};

/// A command for `program` with `args`; the planned program is not used by `Claim::run`.
pub(super) fn command(args: &[&str]) -> Command {
    Command {
        program: Program::Git,
        args: args.iter().map(OsString::from).collect(),
        cwd: std::env::temp_dir(),
        env: Vec::new(),
    }
}

pub(super) fn collecting() -> (Sink, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&seen);
    let sink: Sink = Arc::new(move |line| sink_seen.lock().unwrap().push(line));
    (sink, seen)
}

#[tokio::test]
async fn bld_044_a_real_child_sees_only_the_scrubbed_environment() {
    let running = Running::default();
    let (sink, seen) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    let env = Environment::from_parent(parent_with_secrets());
    let vars = env.for_step(&[("BASE_REF", COMMIT.to_owned())]);
    let kept = claim
        .run(&command(&[]), Path::new("/usr/bin/env"), vars)
        .await
        .unwrap();
    let mut names: Vec<&str> = kept.iter().filter_map(|l| l.split('=').next()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "ANIMA_BUILD_STEP",
            "BASE_REF",
            "CARGO_TERM_COLOR",
            "CI",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_NOSYSTEM",
            "GIT_GRAFT_FILE",
            "GIT_NO_REPLACE_OBJECTS",
            "GIT_TERMINAL_PROMPT",
            "HOME",
            "LANG",
            "NO_COLOR",
            "PATH"
        ]
    );
    assert_eq!(*seen.lock().unwrap(), kept);
    assert!(!kept.iter().any(|line| line.starts_with("DATABASE_URL")));
}

#[tokio::test]
async fn bld_042_a_non_zero_exit_fails_the_step_and_both_streams_reach_the_sink() {
    let running = Running::default();
    let (sink, seen) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    let script = command(&["-c", "echo out; echo err >&2; exit 3"]);
    let result = claim.run(&script, Path::new("/bin/sh"), Vec::new()).await;
    assert_eq!(result, Err(BuilderError::StepFailed));
    let mut lines = seen.lock().unwrap().clone();
    lines.sort_unstable();
    assert_eq!(lines, ["err", "out"]);
}

#[test]
fn bld_029_one_step_runs_at_a_time() {
    let running = Running::default();
    let (sink, _) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    assert!(matches!(
        running.claim(Arc::clone(&sink)),
        Err(BuilderError::Busy)
    ));
    drop(claim);
    assert!(running.claim(sink).is_ok());
}

#[tokio::test]
async fn bld_027_a_step_cancelled_before_it_starts_starts_nothing() {
    let running = Running::default();
    let (sink, seen) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    running.cancel();
    let script = command(&["-c", "echo ran"]);
    let result = claim.run(&script, Path::new("/bin/sh"), Vec::new()).await;
    assert_eq!(result, Err(BuilderError::Cancelled));
    assert!(seen.lock().unwrap().is_empty());
}

pub(super) fn alive(pid: i32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bld_027_cancel_kills_the_whole_process_group() {
    let running = Arc::new(Running::default());
    let (sink, seen) = collecting();
    // The background `sleep` is a grandchild: only a group kill reaches it.
    let script = command(&["-c", "sleep 60 & echo \"$!\"; wait"]);
    let started = Instant::now();
    let step = {
        let running = Arc::clone(&running);
        tokio::spawn(async move {
            let claim = running.claim(Arc::clone(&sink)).unwrap();
            claim.run(&script, Path::new("/bin/sh"), Vec::new()).await
        })
    };
    let grandchild = loop {
        if let Some(line) = seen.lock().unwrap().first() {
            break line.parse::<i32>().unwrap();
        }
        assert!(started.elapsed() < Duration::from_secs(10), "no output");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert!(alive(grandchild));
    running.cancel();
    assert_eq!(step.await.unwrap(), Err(BuilderError::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(10));
    let gone_by = Instant::now() + Duration::from_secs(5);
    while alive(grandchild) && Instant::now() < gone_by {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!alive(grandchild), "the grandchild outlived the cancel");
}

#[tokio::test]
async fn bld_027_a_process_left_behind_by_a_finished_step_is_killed() {
    let running = Running::default();
    let (sink, _) = collecting();
    let claim = running.claim(Arc::clone(&sink)).unwrap();
    // The step exits at once but leaves a sleeper holding its output open.
    let script = command(&["-c", "sleep 60 & echo \"$!\""]);
    let started = Instant::now();
    let kept = claim
        .run(&script, Path::new("/bin/sh"), Vec::new())
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(5));
    let sleeper: i32 = kept[0].parse().unwrap();
    let gone_by = Instant::now() + Duration::from_secs(5);
    while alive(sleeper) && Instant::now() < gone_by {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!alive(sleeper));
}
