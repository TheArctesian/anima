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
    Action::Start {
        thread: thread.to_owned(),
        program: program.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        env: vec![("ONLY".to_owned(), "this".to_owned())],
        folder: "/".to_owned(),
    }
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
