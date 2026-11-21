//! The thread runner (TH11, THR-020 to THR-051): it starts Claude Code and Codex in the holder,
//! drives them over their pipes, keeps the requests that wait for the person, and serves the
//! clients of this computer on its local socket. Nothing of a thread leaves the computer in
//! stage 1b.
//!
//! Pure: [`thread`] (one thread) and [`codex`] (the calls to a Codex server). I/O: this file
//! (the loop and the holder), [`calls`] (the calls of the clients), [`socket`], [`login`],
//! [`record`] and [`log`].

mod calls;
pub mod codex;
pub mod log;
pub mod login;
pub mod record;
pub mod socket;
mod start;
#[cfg(test)]
mod tests;
pub mod thread;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use agent_threads::control::{Limits, Reply, RunState};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::link::{self, HolderLink, LinkError};
use super::wire::{Action, Event, Exit, Held, Line, Outcome, Stream};
use thread::{Step, Thread};

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("the folder of the threads: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Holder(#[from] LinkError),
}

/// What a command to the holder was for, so that its answer goes to the right place.
#[derive(Debug)]
enum Waits {
    Start {
        thread: Uuid,
        call: u64,
        client: mpsc::UnboundedSender<Reply>,
        prompt: String,
    },
    Stop {
        call: u64,
        client: mpsc::UnboundedSender<Reply>,
    },
}

/// The runner: its threads, its link to the holder and its clients.
pub struct Runner {
    dir: PathBuf,
    program: PathBuf,
    link: HolderLink,
    env: Vec<(String, String)>,
    threads: BTreeMap<Uuid, Thread>,
    limits: Limits,
    watchers: Vec<mpsc::UnboundedSender<Reply>>,
    waits: BTreeMap<u64, Waits>,
    /// The last line of each thread that the runner read, so that a line the holder gives
    /// again is not read twice.
    read_up_to: BTreeMap<Uuid, u64>,
    log: log::RunnerLog,
}

/// Runs the runner of the daemon in `daemon_dir` until `stop` completes. `program` is the path
/// of `anima-daemon`, which the runner starts as the holder when none answers. The programs of
/// the threads stay in the holder when the runner ends (THR-113).
///
/// # Errors
/// [`RunnerError`] when the folder, the record, the socket or the holder cannot be set up.
pub async fn run(
    daemon_dir: &Path,
    program: &Path,
    env: Vec<(String, String)>,
    stop: impl Future<Output = ()>,
) -> Result<(), RunnerError> {
    let dir = link::folder(daemon_dir);
    // A socket path has a limit of about 100 bytes (104 on a Mac, 108 on Linux).
    if dir.join(agent_threads::control::SOCKET).as_os_str().len() > 100 {
        return Err(RunnerError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the folder of the daemon is too long for a socket path (more than 100 bytes)",
        )));
    }
    private_folder(&dir)?;
    let saved = record::read(&dir)?;
    let link = link::reach(program, &dir).await?;
    let mut runner = Runner {
        dir: dir.clone(),
        program: program.to_owned(),
        link,
        env,
        threads: BTreeMap::new(),
        limits: saved.limits,
        watchers: Vec::new(),
        waits: BTreeMap::new(),
        read_up_to: BTreeMap::new(),
        log: log::RunnerLog::at(&dir),
    };
    for saved in saved.threads {
        let codex = calls::codex_of(&saved.row, saved.effort);
        let thread = Thread::restore(saved, codex);
        runner.threads.insert(thread.row.id, thread);
    }
    let listener = socket::bind(&dir)?;
    let (sender, mut incoming) = mpsc::channel(256);
    let accepting = tokio::spawn(socket::accept(listener, sender));
    tokio::pin!(stop);
    loop {
        tokio::select! {
            event = runner.link.events.recv() => match event {
                Some(event) => runner.holder_event(event).await,
                None => runner.reconnect().await?,
            },
            Some(call) = incoming.recv() => runner.call(call).await,
            () = &mut stop => break,
        }
    }
    accepting.abort();
    let _ = std::fs::remove_file(dir.join(agent_threads::control::SOCKET));
    runner.save();
    Ok(())
}

fn private_folder(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)?;
    }
    if crate::config::private_file::folder_private(dir)? {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "the folder of the threads is not a private folder of this user",
        ))
    }
}

impl Runner {
    async fn reconnect(&mut self) -> Result<(), RunnerError> {
        tracing::warn!("the link to the holder closed; connecting again");
        self.link = link::reach(&self.program, &self.dir).await?;
        Ok(())
    }

    async fn holder_event(&mut self, event: Event) {
        match event {
            Event::Hello { threads, .. } => self.hello(threads).await,
            Event::Done { id, outcome } => self.done(id, outcome).await,
            Event::Lines {
                thread,
                first,
                lines,
            } => self.lines(&thread, first, lines).await,
            Event::Waiting { .. } => {}
            Event::Exited { thread, exit } => self.exited(&thread, exit).await,
        }
    }

    /// After a connection: a thread that the holder does not hold any more stopped; a program
    /// the holder holds that no thread knows is ended (THR-049).
    async fn hello(&mut self, held: Vec<Held>) {
        let ids: Vec<Uuid> = self.threads.keys().copied().collect();
        for id in ids {
            let found = held.iter().find(|entry| entry.thread == id.to_string());
            let Some(thread) = self.threads.get_mut(&id) else {
                continue;
            };
            if thread.row.state == RunState::Stopped {
                continue;
            }
            let steps = match found {
                None => thread.lost(),
                Some(Held {
                    exit: Some(exit), ..
                }) => thread.exited(exit.code, exit.signal),
                Some(_) => continue,
            };
            self.apply(id, steps).await;
        }
        for entry in held {
            let known = Uuid::parse_str(&entry.thread)
                .ok()
                .and_then(|id| self.threads.get(&id))
                .is_some_and(|thread| thread.row.state != RunState::Stopped);
            if !known {
                let action = if entry.exit.is_some() {
                    Action::Forget {
                        thread: entry.thread,
                    }
                } else {
                    Action::Stop {
                        thread: entry.thread,
                    }
                };
                self.command(action).await;
            }
        }
        self.save();
    }

    async fn lines(&mut self, key: &str, first: u64, lines: Vec<Line>) {
        let Some(id) = Uuid::parse_str(key)
            .ok()
            .filter(|id| self.threads.contains_key(id))
        else {
            return;
        };
        let mut last = self.read_up_to.get(&id).copied().unwrap_or(0);
        for (at, line) in lines.into_iter().enumerate() {
            let seq = first + at as u64;
            if seq <= last {
                continue;
            }
            last = seq;
            let Some(thread) = self.threads.get_mut(&id) else {
                return;
            };
            let steps = match line.stream {
                Stream::Out => thread.line(&line.text, &mut Uuid::now_v7),
                Stream::Err => {
                    thread.error_line(&line.text);
                    Vec::new()
                }
            };
            self.apply(id, steps).await;
        }
        self.read_up_to.insert(id, last);
        // The record holds the open requests before the holder drops their lines (THR-030).
        self.save();
        self.command(Action::Ack {
            thread: key.to_owned(),
            seq: last,
        })
        .await;
    }

    async fn exited(&mut self, key: &str, exit: Exit) {
        let Some(id) = Uuid::parse_str(key).ok() else {
            return;
        };
        let Some(thread) = self.threads.get_mut(&id) else {
            return;
        };
        let steps = thread.exited(exit.code, exit.signal);
        self.apply(id, steps).await;
        self.save();
        self.command(Action::Forget {
            thread: key.to_owned(),
        })
        .await;
    }

    /// Does the steps of a thread: lines to its program, an end of its children, updates.
    async fn apply(&mut self, id: Uuid, steps: Vec<Step>) {
        for step in steps {
            match step {
                Step::Write(line) => {
                    self.command(Action::Write {
                        thread: id.to_string(),
                        line,
                    })
                    .await;
                }
                Step::EndChildren => {
                    self.command(Action::EndChildren {
                        thread: id.to_string(),
                    })
                    .await;
                }
                Step::Change(change) => {
                    let Some(thread) = self.threads.get_mut(&id) else {
                        continue;
                    };
                    let update = thread.note(change);
                    let reply = Reply::Update(Box::new(update));
                    self.watchers
                        .retain(|watcher| watcher.send(reply.clone()).is_ok());
                }
            }
        }
    }

    /// Sends a command to the holder; a closed link is found by the loop and connected again.
    async fn command(&mut self, action: Action) -> Option<u64> {
        match self.link.send(action).await {
            Ok(id) => Some(id),
            Err(error) => {
                tracing::warn!(%error, "the holder did not take a command");
                None
            }
        }
    }

    fn save(&self) {
        let record = record::Record {
            limits: self.limits,
            threads: self.threads.values().map(Thread::saved).collect(),
        };
        if let Err(error) = record::write(&self.dir, &record) {
            tracing::warn!(%error, "the record of the runner was not written");
        }
    }

    fn running(&self) -> u32 {
        let count = self
            .threads
            .values()
            .filter(|thread| thread.row.state != RunState::Stopped)
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

/// Whether a holder answer is a refusal.
const fn refused(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Refused { .. })
}
