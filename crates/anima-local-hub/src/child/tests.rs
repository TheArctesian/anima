use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Duration;

use tokio::sync::mpsc;

use super::{Child, ChildError, Spec, Stop, Stopped};
use crate::log_file::LogFile;

const SOON: Duration = Duration::from_secs(20);

fn shell(script: &str) -> Spec {
    Spec {
        name: "member",
        program: "/bin/sh".into(),
        args: vec![OsString::from("-c"), OsString::from(script)],
        env: vec![("ONLY", OsString::from("this"))],
    }
}

fn start(folder: &Path, script: &str) -> (Child, mpsc::UnboundedReceiver<&'static str>) {
    let log = LogFile::open(&folder.join("member.log"), 1 << 20).unwrap();
    let (ended, names) = mpsc::unbounded_channel();
    (Child::start(&shell(script), log, ended).unwrap(), names)
}

fn log_of(folder: &Path) -> String {
    fs::read_to_string(folder.join("member.log")).unwrap()
}

#[tokio::test]
async fn a_member_gets_only_the_environment_it_is_given() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, _names) = start(folder.path(), "/usr/bin/env; echo done");

    child
        .line_where(SOON, |line| (line == "done").then_some(()))
        .await
        .unwrap();

    let log = log_of(folder.path());
    let names: Vec<&str> = log
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        // The shell adds these itself.
        .filter(|name| !["PWD", "SHLVL", "_", "OLDPWD"].contains(name))
        .collect();
    assert_eq!(names, ["ONLY"], "{log}");
}

#[tokio::test]
async fn both_outputs_go_to_the_log_and_the_ready_line_is_found_on_either() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, _names) = start(
        folder.path(),
        "echo starting; echo 'listening on 127.0.0.1:7' >&2; sleep 30",
    );

    let address = child
        .line_where(SOON, |line| {
            line.strip_prefix("listening on ").map(str::to_owned)
        })
        .await
        .unwrap();

    assert_eq!(address, "127.0.0.1:7");
    assert!(log_of(folder.path()).contains("starting\n"));
    assert!(matches!(child.stop(SOON).await.unwrap(), Stopped::Ended(_)));
}

#[tokio::test]
async fn a_member_that_ends_before_it_is_ready_is_an_error_with_its_status() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, mut names) = start(folder.path(), "echo no; exit 11");

    let error = child
        .line_where(SOON, |line| (line == "ready").then_some(()))
        .await
        .unwrap_err();

    assert_eq!(error.exit_code(), Some(11), "{error}");
    assert_eq!(names.recv().await, Some("member"));
}

#[tokio::test]
async fn a_member_that_is_never_ready_is_an_error_after_the_limit() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, _names) = start(folder.path(), "sleep 30");

    let error = child
        .line_where(Duration::from_millis(200), |line| {
            (line == "ready").then_some(())
        })
        .await
        .unwrap_err();

    assert!(matches!(error, ChildError::NotReady { .. }), "{error}");
    child.stop(SOON).await.unwrap();
}

#[tokio::test]
async fn a_member_that_ignores_sigterm_is_killed_after_the_limit() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, mut names) = start(
        folder.path(),
        "trap '' TERM; echo up; while :; do sleep 0.05; done",
    );
    child
        .line_where(SOON, |line| (line == "up").then_some(()))
        .await
        .unwrap();

    let stopped = child.stop(Duration::from_millis(300)).await.unwrap();

    assert_eq!(stopped, Stopped::Killed);
    assert_eq!(names.recv().await, Some("member"));
}

#[tokio::test]
async fn output_after_the_ready_line_still_goes_to_the_log_when_nobody_reads_it() {
    let folder = tempfile::tempdir().unwrap();
    let (mut child, _names) = start(
        folder.path(),
        "echo ready; i=0; while [ $i -lt 3000 ]; do echo \"line $i of plenty of text to fill a pipe\"; i=$((i+1)); done; echo end",
    );
    child
        .line_where(SOON, |line| (line == "ready").then_some(()))
        .await
        .unwrap();
    child.stop_reading();

    child.end_within(SOON).await.unwrap();

    // The copy tasks finish after the exit is seen.
    for _ in 0..100 {
        if log_of(folder.path()).ends_with("end\n") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the log does not hold the last line");
}

#[tokio::test]
async fn a_program_that_is_not_there_cannot_be_started() {
    let folder = tempfile::tempdir().unwrap();
    let log = LogFile::open(&folder.path().join("member.log"), 1 << 20).unwrap();
    let (ended, _names) = mpsc::unbounded_channel();
    let mut spec = shell("true");
    spec.program = folder.path().join("absent");

    let error = Child::start(&spec, log, ended).unwrap_err();

    assert!(matches!(error, ChildError::Start { .. }), "{error}");
}

#[tokio::test]
async fn a_stop_that_was_asked_before_anybody_waits_is_kept() {
    let (ask, mut stop) = Stop::new();
    assert!(!stop.is_asked());

    ask.send_replace(true);

    assert!(stop.is_asked());
    tokio::time::timeout(SOON, stop.asked()).await.unwrap();
}
