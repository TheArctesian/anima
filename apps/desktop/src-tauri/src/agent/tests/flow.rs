//! The agent runs: copied, then enrolled with the code on standard input, then installed; a
//! failure, a deadline or the app's exit stops the chain and leaves no process behind.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use super::{
    CODE,
    rig::{NEVER, Rig, WIDE, names_in, rig},
};
use crate::agent::AgentError;

#[test]
fn met_019_the_agent_is_copied_then_enrolled_with_the_code_on_stdin_then_installed() {
    let rig = rig("exit 0");
    assert_eq!(rig.install(), Ok(rig.installed()));
    let installed = rig.installed();
    assert_eq!(fs::read(&installed).ok(), fs::read(&rig.source).ok());
    let mode = fs::metadata(&installed)
        .expect("installed")
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o755);
    let enrol = rig.run_of("enrol --hub https://hub.test -", &format!("{CODE}\n"));
    assert_eq!(rig.runs(), enrol + &rig.run_of("install", ""));
    // The copy went in by rename: no part file is left beside it.
    assert_eq!(
        names_in(installed.parent().expect("a folder")),
        ["anima-agent"]
    );
}

#[test]
fn met_019_a_failed_enrolment_stops_before_install() {
    let rig = rig("[ \"$1\" = enrol ] && exit 3; exit 0");
    assert_eq!(rig.install(), Err(AgentError::Enrol));
    assert!(!rig.runs().contains("args:install"), "{}", rig.runs());
}

#[test]
fn met_019_a_failed_install_after_enrolment_is_its_own_error() {
    let rig = rig("[ \"$1\" = install ] && exit 1; exit 0");
    assert_eq!(rig.install(), Err(AgentError::Install));
    assert!(rig.runs().contains("args:enrol"), "{}", rig.runs());
}

/// A stand-in that writes its pid to `pid` and then sleeps.
fn slow_rig() -> (Rig, PathBuf) {
    let rig = rig("");
    let pid_file = rig.dir.path().join("pid");
    let script = format!(
        "{}\necho $$ > '{}'; exec /bin/sleep 30\n",
        fs::read_to_string(&rig.source).expect("the stand-in"),
        pid_file.display()
    );
    fs::write(&rig.source, script).expect("the slow stand-in");
    (rig, pid_file)
}

/// Whether the process in `pid_file` is gone; one only killed would still be a zombie.
fn reaped(pid_file: &Path) -> bool {
    let pid = fs::read_to_string(pid_file).expect("the stand-in wrote its pid");
    let listed = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .output()
        .expect("ps");
    listed.stdout.is_empty()
}

#[test]
fn met_019_an_agent_past_its_deadline_is_killed_and_reaped() {
    let (rig, pid_file) = slow_rig();
    let started = Instant::now();
    let answer = rig.install_within(Duration::from_secs(2), &NEVER);
    assert_eq!(answer, Err(AgentError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(reaped(&pid_file), "the agent is still there");
    assert!(!rig.runs().contains("args:install"));
}

#[test]
fn met_019_an_agent_running_when_the_app_exits_is_killed_and_reaped() {
    let (rig, pid_file) = slow_rig();
    let stop = AtomicBool::new(false);
    let started = Instant::now();
    let answer = thread::scope(|scope| {
        scope.spawn(|| {
            while !pid_file.exists() && started.elapsed() < Duration::from_secs(10) {
                thread::sleep(Duration::from_millis(10));
            }
            stop.store(true, Ordering::Release);
        });
        rig.install_within(WIDE, &stop)
    });
    assert_eq!(answer, Err(AgentError::Failed));
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the stop was not seen"
    );
    assert!(reaped(&pid_file), "the agent is still there");
}
