//! The probes run against small `sh` scripts in a temporary folder, never the developer tools of
//! the machine that runs the tests.

use std::fs;
use std::time::Instant;

use daemon_core::inventory::{Reason, Tool, ToolName, Verdict};
use rustix::process::{Pid, test_kill_process};

use super::*;

/// The `PATH` the scripts get: enough for `sleep`, `env` and `ls`.
const SCRIPT_PATH: &str = "/bin:/usr/bin";

/// An executable `sh` script `name` in `folder` that runs `body`.
fn script(folder: &Path, name: &str, body: &str) -> PathBuf {
    let path = folder.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

/// A search of only `folders`, on Linux.
fn search(folders: &[&Path]) -> Search {
    Search {
        folders: folders.iter().map(|folder| folder.to_path_buf()).collect(),
        path: OsString::from(SCRIPT_PATH),
        os: Os::Linux,
        developer_tools: false,
    }
}

const fn program(name: &'static str) -> Program {
    Program {
        name,
        args: &["--version"],
    }
}

async fn printed(name: &'static str, own_home: bool, search: &Search) -> String {
    match run(program(name), own_home, search).await {
        Outcome::Printed(output) => String::from_utf8(output).expect("text"),
        other => panic!("{name} gave {other:?}"),
    }
}

/// Whether the process whose id is in `pid_file` stops running within 3 seconds.
async fn stopped(pid_file: &Path) -> bool {
    let text = fs::read_to_string(pid_file).expect("the script wrote its pid");
    let pid = Pid::from_raw(text.trim().parse().expect("a pid")).expect("not 0");
    for _ in 0..30 {
        if test_kill_process(pid) == Err(rustix::io::Errno::SRCH) || exited_child(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// The Linux desktop builder adopts orphaned children. An exited child may keep its PID
/// until that builder reaps it, so also read the kernel's process state.
#[cfg(target_os = "linux")]
fn exited_child(pid: Pid) -> bool {
    fs::read_to_string(format!("/proc/{}/stat", pid.as_raw_pid())).is_ok_and(|stat| {
        matches!(
            stat.rsplit_once(") ")
                .and_then(|(_, fields)| fields.as_bytes().first()),
            Some(b'Z' | b'X')
        )
    })
}

#[cfg(not(target_os = "linux"))]
const fn exited_child(_pid: Pid) -> bool {
    false
}

#[tokio::test]
async fn cap_021_a_program_is_the_first_executable_file_of_that_name() {
    let first = tempfile::tempdir().expect("a folder");
    let second = tempfile::tempdir().expect("a folder");
    fs::write(first.path().join("git"), "not executable").expect("written");
    fs::create_dir(first.path().join("bun")).expect("a folder named bun");
    let git = script(second.path(), "git", "echo git version 2.43.0");
    let found = search(&[first.path(), second.path()]);
    assert_eq!(find("git", &found).await, Some(git));
    assert_eq!(find("bun", &found).await, None);
    assert_eq!(
        run(program("anima-no-such-tool"), false, &search(&[])).await,
        Outcome::Missing
    );
}

#[tokio::test]
async fn cap_022_a_probe_gets_no_home_no_input_and_only_its_fixed_environment() {
    let folder = tempfile::tempdir().expect("a folder");
    script(
        folder.path(),
        "probe",
        "echo \"home=${HOME-none} pwd=$(pwd) args=$*\"; if read line; then echo input; fi\n\
         env | cut -d= -f1 | sort | tr '\\n' ' '; echo\n\
         echo \"$LC_ALL $RUSTUP_AUTO_INSTALL $PATH\"",
    );
    let output = printed("probe", false, &search(&[folder.path()])).await;
    let mut lines = output.lines();
    assert_eq!(lines.next(), Some("home=none pwd=/ args=--version"));
    let names: Vec<&str> = lines
        .next()
        .expect("the names")
        .split_whitespace()
        .collect();
    let shell_own = ["OLDPWD", "PWD", "SHLVL", "_"];
    let passed: Vec<&str> = names
        .into_iter()
        .filter(|name| !shell_own.contains(name))
        .collect();
    assert_eq!(passed, ["LC_ALL", "PATH", "RUSTUP_AUTO_INSTALL"]);
    assert_eq!(lines.next(), Some("C 0 /bin:/usr/bin"));
}

#[tokio::test]
async fn cap_025_claude_and_codex_get_an_empty_home_of_their_own_that_is_removed() {
    let folder = tempfile::tempdir().expect("a folder");
    script(
        folder.path(),
        "probe",
        "echo \"$HOME|$CODEX_HOME|$CLAUDE_CONFIG_DIR|$(ls -A \"$HOME\")\"\n\
         mkdir \"$HOME/.codex\" && echo written > \"$HOME/.codex/state\"",
    );
    let found = search(&[folder.path()]);
    let output = printed("probe", true, &found).await;
    let fields: Vec<&str> = output.trim_end().split('|').collect();
    let [home, codex_home, claude_home, listed] = fields[..] else {
        panic!("{output}");
    };
    assert!(Path::new(home).is_absolute(), "{output}");
    assert_eq!((codex_home, claude_home), (home, home));
    assert_eq!(listed, "", "the home folder starts empty");
    assert!(!Path::new(home).exists(), "the home folder is removed");
    let again = printed("probe", true, &found).await;
    assert_ne!(again, output, "each run gets a new folder");
}

#[tokio::test]
async fn cap_024_a_probe_that_fails_or_cannot_start_has_exited() {
    let folder = tempfile::tempdir().expect("a folder");
    script(folder.path(), "broken", "echo 1.0; exit 3");
    fs::write(folder.path().join("nonsense"), "\u{7f}ELF?").expect("written");
    fs::set_permissions(
        folder.path().join("nonsense"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("executable");
    let found = search(&[folder.path()]);
    assert_eq!(run(program("broken"), false, &found).await, Outcome::Exited);
    assert_eq!(
        run(program("nonsense"), false, &found).await,
        Outcome::Exited
    );
}

#[tokio::test]
async fn cap_024_a_stuck_probe_times_out_and_it_and_its_background_child_are_killed() {
    let folder = tempfile::tempdir().expect("a folder");
    let probe_pid = folder.path().join("probe.pid");
    let child_pid = folder.path().join("child.pid");
    script(
        folder.path(),
        "stuck",
        &format!(
            "sleep 30 & echo $! > '{}'; echo $$ > '{}'; sleep 30",
            child_pid.display(),
            probe_pid.display()
        ),
    );
    let started = Instant::now();
    let outcome = run(program("stuck"), false, &search(&[folder.path()])).await;
    assert_eq!(outcome, Outcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(stopped(&probe_pid).await, "the probe stopped");
    assert!(stopped(&child_pid).await, "its background child stopped");
}

#[tokio::test]
async fn cap_022_a_probe_ends_when_it_exits_even_if_its_background_child_holds_its_output() {
    let folder = tempfile::tempdir().expect("a folder");
    let child_pid = folder.path().join("child.pid");
    script(
        folder.path(),
        "quick",
        &format!("sleep 30 & echo $! > '{}'; echo 1.2.3", child_pid.display()),
    );
    let started = Instant::now();
    let output = printed("quick", false, &search(&[folder.path()])).await;
    assert_eq!(output, "1.2.3\n");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(stopped(&child_pid).await, "its background child stopped");
}

#[tokio::test]
async fn cap_020_the_probes_of_a_system_make_its_inventory() {
    let folder = tempfile::tempdir().expect("a folder");
    script(folder.path(), "git", "echo git version 2.43.0");
    script(folder.path(), "bun", "exit 1");
    script(folder.path(), "node", "echo hello");
    script(folder.path(), "codex", "echo \"codex-cli 0.169.0\"");
    script(
        folder.path(),
        "rustup",
        "echo x86_64-unknown-linux-gnu; echo wasm32-unknown-unknown",
    );
    let system = (Architecture::X86_64, Os::Linux, "24.04".to_owned());
    let inventory = probe_all(system, &search(&[folder.path()])).await;
    let tool = |name, verdict| Tool { name, verdict };
    assert_eq!(
        inventory.tools,
        [
            tool(ToolName::Git, Verdict::Usable("2.43.0".to_owned())),
            tool(ToolName::Bun, Verdict::NotUsable(Reason::Exited)),
            tool(ToolName::Node, Verdict::NotUsable(Reason::NoVersion)),
            tool(ToolName::Codex, Verdict::Usable("0.169.0".to_owned())),
        ]
    );
    assert_eq!(
        inventory.build_targets,
        ["wasm32-unknown-unknown", "x86_64-unknown-linux-gnu"]
    );
}
