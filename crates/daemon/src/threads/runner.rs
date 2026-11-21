//! The thread runner (TH11, THR-020 to THR-051): it starts Claude Code and Codex in the holder,
//! drives them over their pipes, keeps the requests that wait for the person, and serves the
//! clients of this computer on its local socket. Nothing of a thread leaves the computer in
//! stage 1b.
//!
//! Pure: [`thread`] (one thread) and [`codex`] (the calls to a Codex server). I/O: this file
//! (the loop and the holder), [`holder_events`], [`calls`] (the calls of the clients),
//! [`socket`], [`login`], [`record`] and [`log`].

mod calls;
pub mod codex;
mod holder_events;
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
use std::time::Duration;

use agent_threads::control::{Limits, Outcome, Refusal, Reply, RunState};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::link::{self, HolderLink, LinkError};
use super::wire::Action;
use log::Kind;
use socket::{Caller, Client};
use thread::{Step, Thread};

/// The lock file of the runner in the folder of the threads: one runner for one folder.
const LOCK: &str = "runner.lock";
/// The most stopped threads that the runner keeps; the oldest go first.
const STOPPED_KEPT: usize = 50;
/// The longest wait between two tries to reach the holder.
const RETRY_MOST: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("the folder of the threads: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Holder(#[from] LinkError),
    #[error("a different runner holds the threads of this daemon")]
    Busy,
}

/// What a command to the holder was for, so that its answer goes to the right place.
#[derive(Debug)]
enum Waits {
    Start {
        thread: Uuid,
        back: Caller,
        prompt: String,
    },
    Stop {
        back: Caller,
    },
    /// The last line of a prompt; the prompt is shown as sent when the holder takes it.
    Send {
        thread: Uuid,
        back: Caller,
        text: String,
    },
    /// The line of an answer; the request closes when the holder takes it (THR-030).
    Answer(Pending),
}

/// An answer that waits for the holder to take its line.
#[derive(Debug)]
struct Pending {
    thread: Uuid,
    request: Uuid,
    word: &'static str,
    back: Caller,
}

/// The link to the holder was lost while a command was sent.
#[derive(Debug)]
struct LinkDown;

/// The runner: its threads, its link to the holder and its clients.
pub struct Runner {
    dir: PathBuf,
    program: PathBuf,
    link: HolderLink,
    /// The holder speaks the version of this runner; until it says so, nothing starts.
    holder_fits: bool,
    env: Vec<(String, String)>,
    threads: BTreeMap<Uuid, Thread>,
    limits: Limits,
    watchers: Vec<Client>,
    waits: BTreeMap<u64, Waits>,
    /// The last line of each thread that the runner read, so that a line the holder gives
    /// again is not read twice.
    read_up_to: BTreeMap<Uuid, u64>,
    log: log::RunnerLog,
}

/// Whether the daemon in `daemon_dir` serves a person, so that it may run agents in the login
/// of that person: never the system unit, which runs as an account of its own (THR-128).
#[must_use]
pub fn serves_a_person(daemon_dir: &Path) -> bool {
    daemon_dir != Path::new(crate::units::plan::SYSTEM_HOME) && !rustix::process::getuid().is_root()
}

/// Runs the runner of the daemon in `daemon_dir` until `stop` completes. `program` is the path
/// of `anima-daemon`, which the runner starts as the holder when none answers. The programs of
/// the threads stay in the holder when the runner ends (THR-113).
///
/// # Errors
/// [`RunnerError`] when the folder, its lock, the record or the socket cannot be set up.
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
    let _lock = lock(&dir)?;
    let saved = record::read(&dir)?;
    tokio::pin!(stop);
    let link = tokio::select! {
        link = reach_patiently(program, &dir) => link,
        () = &mut stop => return Ok(()),
    };
    let mut runner = Runner {
        dir: dir.clone(),
        program: program.to_owned(),
        link,
        holder_fits: false,
        env,
        threads: BTreeMap::new(),
        limits: saved.limits,
        watchers: Vec::new(),
        waits: BTreeMap::new(),
        read_up_to: BTreeMap::new(),
        log: log::RunnerLog::at(&dir),
    };
    runner.restore(saved.threads);
    let listener = socket::bind(&dir)?;
    runner.serve(listener, &mut stop).await;
    if let Err(error) = std::fs::remove_file(dir.join(agent_threads::control::SOCKET)) {
        tracing::warn!(%error, "the socket of the runner was not removed");
    }
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

/// Takes the lock of the folder: a second runner on the same folder ends at once, so that two
/// runners never stop the programs of each other (THR-113).
fn lock(dir: &Path) -> Result<std::fs::File, RunnerError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(dir.join(LOCK))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| RunnerError::Busy)?;
    Ok(file)
}

/// The link to the holder; while none answers, it tries again, each time a little later.
async fn reach_patiently(program: &Path, dir: &Path) -> HolderLink {
    let mut wait = Duration::from_secs(1);
    loop {
        match link::reach(program, dir).await {
            Ok(link) => return link,
            Err(error) => tracing::warn!(%error, "the holder did not answer; trying again"),
        }
        tokio::time::sleep(wait).await;
        wait = (wait * 2).min(RETRY_MOST);
    }
}

impl Runner {
    fn restore(&mut self, saved: Vec<thread::Saved>) {
        for saved in saved {
            let codex = calls::codex_of(&saved.row, saved.effort);
            let thread = Thread::restore(saved, codex);
            self.threads.insert(thread.row.id, thread);
        }
    }

    /// The loop of the runner: the holder, the clients, and the stop.
    async fn serve(&mut self, listener: tokio::net::UnixListener, stop: impl Future<Output = ()>) {
        let (sender, mut incoming) = mpsc::channel(256);
        let accepting = tokio::spawn(socket::accept(listener, sender));
        tokio::pin!(stop);
        loop {
            tokio::select! {
                event = self.link.events.recv() => match event {
                    Some(event) => self.holder_event(event).await,
                    None => self.reconnect().await,
                },
                Some(call) = incoming.recv() => self.call(call).await,
                () = &mut stop => break,
            }
        }
        accepting.abort();
    }

    /// A new link to the holder. Each call that waited for a command of the old link gets its
    /// answer now: a start waits for the holder to say what it holds; the rest are refused.
    async fn reconnect(&mut self) {
        tracing::warn!("the link to the holder closed; connecting again");
        self.holder_fits = false;
        let waits = std::mem::take(&mut self.waits);
        for (command, waits) in waits {
            match waits {
                start @ Waits::Start { .. } => {
                    self.waits.insert(command, start);
                }
                other => self.refuse_wait(other, Refusal::Holder),
            }
        }
        self.link = reach_patiently(&self.program, &self.dir).await;
    }

    /// Answers a call that waited for the holder with a refusal; an answer that waited may be
    /// given again.
    fn refuse_wait(&mut self, waits: Waits, why: Refusal) {
        let back = match waits {
            Waits::Start { back, .. } | Waits::Stop { back } | Waits::Send { back, .. } => back,
            Waits::Answer(pending) => {
                if let Some(thread) = self.threads.get_mut(&pending.thread) {
                    thread.answer_failed(pending.request);
                }
                pending.back
            }
        };
        back.answer(Outcome::Refused { why });
    }

    /// Does the steps of a thread, in order. Gives the ID of the command of the last line
    /// written, or [`LinkDown`] when a command did not reach the holder.
    async fn apply(&mut self, id: Uuid, steps: Vec<Step>) -> Result<Option<u64>, LinkDown> {
        let key = id.to_string();
        let mut last_write = None;
        for step in steps {
            let action = match step {
                Step::Write(line) => Action::Write {
                    thread: key.clone(),
                    line,
                },
                Step::Mark => Action::Mark {
                    thread: key.clone(),
                },
                Step::EndChildren => Action::EndChildren {
                    thread: key.clone(),
                },
                Step::EndProgram => Action::Stop {
                    thread: key.clone(),
                },
                Step::Log(what) => {
                    self.note(Kind::Stop, Some(id), what);
                    continue;
                }
                Step::Change(change) => {
                    self.publish(id, change);
                    continue;
                }
            };
            let write = matches!(action, Action::Write { .. });
            let command = self.command(action).await.ok_or(LinkDown)?;
            if write {
                last_write = Some(command);
            }
        }
        Ok(last_write)
    }

    /// Gives a change of a thread its number and sends it to each watcher; a watcher that does
    /// not read is let go (THR-045).
    fn publish(&mut self, id: Uuid, change: agent_threads::control::Change) {
        let Some(thread) = self.threads.get_mut(&id) else {
            return;
        };
        let reply = Reply::Update(Box::new(thread.note(change)));
        self.watchers
            .retain(|watcher| socket::tell(watcher, reply.clone()));
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

    /// Writes the record, after it lets go of the oldest stopped threads.
    fn save(&mut self) {
        let mut stopped: Vec<(chrono::DateTime<chrono::Utc>, Uuid)> = self
            .threads
            .values()
            .filter(|thread| thread.row.state == RunState::Stopped)
            .map(|thread| (thread.row.started, thread.row.id))
            .collect();
        stopped.sort();
        let over = stopped.len().saturating_sub(STOPPED_KEPT);
        for (_, id) in stopped.into_iter().take(over) {
            self.threads.remove(&id);
            self.read_up_to.remove(&id);
        }
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
