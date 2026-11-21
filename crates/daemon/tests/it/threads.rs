//! The holder of the agent programs (THR-113, THR-049, THR-040, THR-128), as a real process of
//! `anima-daemon holder` with made programs in place of the agents.

use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process};
use std::time::Duration;

use anima_daemon::threads::link::HolderLink;
use anima_daemon::threads::tree;
use anima_daemon::threads::wire::{Action, Event, Line, Outcome};
use tempfile::TempDir;

mod connection;
mod holder_update;
mod requests;
mod runner;
mod start;
mod stop;

/// A holder process in a folder of its own, ended with the test.
struct Holder {
    child: Child,
    dir: PathBuf,
    _keep: TempDir,
}

impl Holder {
    fn start(limit: usize) -> Self {
        let keep = tempfile::tempdir().expect("a folder");
        let dir = keep.path().join("threads");
        let child = spawn(&dir, limit);
        Self {
            child,
            dir,
            _keep: keep,
        }
    }

    async fn link(&self) -> Talk {
        for _ in 0..100 {
            if let Ok(link) = HolderLink::connect(&self.dir).await {
                return Talk {
                    link,
                    log: Vec::new(),
                    cursor: 0,
                };
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the holder did not listen");
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(dir: &Path, limit: usize) -> Child {
    Process::new(env!("CARGO_BIN_EXE_anima-daemon"))
        .arg("holder")
        .arg(dir)
        .arg(limit.to_string())
        .env_clear()
        .spawn()
        .expect("the holder starts")
}

fn start(thread: &str, program: &str, args: &[&str]) -> Action {
    let mut env = vec![("ONLY".to_owned(), "this".to_owned())];
    let program = match stand_ins() {
        Some(folder) if program == "/bin/sh" => {
            env.push((
                "PATH".to_owned(),
                format!("{}:/usr/bin:/bin", folder.display()),
            ));
            folder.join("sh").display().to_string()
        }
        _ => program.to_owned(),
    };
    Action::Start {
        thread: thread.to_owned(),
        program,
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        env,
        folder: "/".to_owned(),
    }
}

/// On a Mac, the folder of the made shell and the programs its scripts run. `ps -E` does not
/// show the environment of a program of the system, so a stop finds the mark of a thread only on
/// copies signed here; with them the Mac reads each mark, as Linux does (THR-049). `None` on
/// Linux, where each process of the user is read.
#[cfg_attr(
    target_os = "macos",
    expect(
        clippy::unnecessary_wraps,
        reason = "None on Linux, where no stand-in is needed"
    )
)]
fn stand_ins() -> Option<&'static Path> {
    #[cfg(target_os = "macos")]
    {
        static FOLDER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        Some(FOLDER.get_or_init(made_stand_ins).as_path())
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Signs copies of `bash` (as `sh`), `perl` and `sleep` once for each build folder; each test
/// process makes its own and the first rename wins.
#[cfg(target_os = "macos")]
fn made_stand_ins() -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("mac-stand-ins");
    if folder.join("sleep").exists() {
        return folder;
    }
    let made = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a folder");
    for (name, from) in [
        ("sh", "/bin/bash"),
        ("perl", "/usr/bin/perl"),
        ("sleep", "/bin/sleep"),
    ] {
        let copy = made.path().join(name);
        std::fs::copy(from, &copy).expect("a copy");
        let signed = Process::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&copy)
            .output()
            .expect("codesign runs");
        assert!(signed.status.success(), "{name} is signed");
    }
    // A different test process may have made the folder first: then its copies are used.
    let kept = made.keep();
    if std::fs::rename(&kept, &folder).is_err() {
        let _ = std::fs::remove_dir_all(&kept);
    }
    assert!(folder.join("sleep").exists(), "the stand-ins are in place");
    folder
}

/// A link with each event it got, so that an event that comes before an answer is not lost.
struct Talk {
    link: HolderLink,
    log: Vec<Event>,
    cursor: usize,
}

impl Talk {
    async fn send(&mut self, action: Action) -> u64 {
        self.link.send(action).await.expect("sent")
    }

    async fn more(&mut self, deadline: tokio::time::Instant) {
        let event = tokio::time::timeout_at(deadline, self.link.events.recv())
            .await
            .unwrap_or_else(|_| panic!("nothing in time; seen {:?}", &self.log[self.cursor..]))
            .expect("the holder is there");
        self.log.push(event);
    }

    /// The events after the last call, up to the first for which `last` is true.
    async fn until(&mut self, last: impl Fn(&Event) -> bool) -> Vec<Event> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(at) = self.log[self.cursor..].iter().position(&last) {
                let found = self.log[self.cursor..=self.cursor + at].to_vec();
                self.cursor += at + 1;
                return found;
            }
            self.more(deadline).await;
        }
    }

    /// The answer to the command `id`, wherever it is in the log.
    async fn answer(&mut self, id: u64) -> Outcome {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let found = self.log.iter().find_map(|event| match event {
                Event::Done { id: done, outcome } if *done == id => Some(outcome.clone()),
                _ => None,
            });
            if let Some(outcome) = found {
                return outcome;
            }
            self.more(deadline).await;
        }
    }
}

fn texts(events: &[Event]) -> Vec<(u64, String)> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Lines { first, lines, .. } => lines
                .iter()
                .enumerate()
                .map(|(at, Line { text, .. })| (first + at as u64, text.clone()))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// Whether `pid` is gone within 3 seconds: a process that ended is reaped by its new parent a
/// short time later, and until then a test of a signal still finds it.
async fn gone(pid: u32) -> bool {
    for _ in 0..60 {
        if !tree::alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

fn exited(thread: &str) -> impl Fn(&Event) -> bool + '_ {
    move |event| matches!(event, Event::Exited { thread: ended, .. } if ended == thread)
}
