//! The holder (THR-113): a small process that owns the agent programs of the threads and the end
//! of their output, so that the daemon can stop, be updated and start again while agents run.
//! It is `anima-daemon holder <folder>`, started by the daemon with an empty environment. It
//! reads no settings, opens no file of the person and speaks to no network.
//!
//! Its channel is one socket file in a folder of mode 0700 of the user (THR-128). It checks that
//! the peer is the same user. It cannot tell the daemon from a different program of the same
//! user (OD-THR18): the channel keeps out a different user, a web page and the network, no more.
//! A new connection takes the place of the one before it, so that a daemon that starts again
//! is never locked out by the one that died.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use super::outbox::{LIMIT, Outbox};
use super::program::Input;
use super::record::{self, Record};
use super::wire::{self, Action, Command, Event, Exit, Held, MAX_FRAME, Outcome, ThreadKey};

/// The name of the socket file in the folder of the holder.
pub const SOCKET: &str = "holder.sock";
const LOCK: &str = "holder.lock";

/// How many answers the holder keeps, so that a command sent again gets its first answer.
const DONE_KEPT: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum HolderError {
    #[error("the folder of the holder: {0}")]
    Io(#[from] io::Error),
    #[error("the folder of the holder is not a private folder of this user")]
    Exposed,
    #[error("a different holder holds this folder")]
    AlreadyRunning,
}

/// One thread in the holder.
pub struct Slot {
    pub pid: u32,
    pub started: Option<String>,
    pub input: Input,
    pub outbox: Outbox,
    pub waiting: bool,
    pub exit: Option<Exit>,
}

/// What the tasks of the holder share.
pub struct Shared {
    pub threads: BTreeMap<ThreadKey, Slot>,
    pub dir: PathBuf,
    connection: Option<(u64, mpsc::UnboundedSender<String>)>,
    connections: u64,
    pub(super) done: BTreeMap<u64, Outcome>,
    done_order: VecDeque<u64>,
    /// The daemon speaks a different version: no new start (THR-113).
    pub draining: bool,
    pub limit: usize,
}

impl Shared {
    /// Gives an event to the daemon if one is connected; else the outbox keeps the lines.
    pub fn send(&self, event: &Event) {
        if let Some((_, sender)) = &self.connection {
            let _ = sender.send(wire::frame(event));
        }
    }

    /// Writes the record of the programs, so that a new holder ends what an old one left (H9).
    pub fn write_record(&self) {
        let running = self
            .threads
            .iter()
            .filter(|(_, slot)| slot.exit.is_none())
            .map(|(thread, slot)| record::Entry {
                thread: thread.clone(),
                pid: slot.pid,
                started: slot.started.clone(),
            });
        let _ = record::write(
            &self.dir,
            &Record {
                version: wire::VERSION,
                threads: running.collect(),
            },
        );
    }

    pub fn remember(&mut self, id: u64, outcome: Outcome) {
        self.done.insert(id, outcome);
        self.done_order.push_back(id);
        while self.done_order.len() > DONE_KEPT {
            if let Some(old) = self.done_order.pop_front() {
                self.done.remove(&old);
            }
        }
    }

    #[must_use]
    pub fn held(&self) -> Vec<Held> {
        self.threads
            .iter()
            .map(|(thread, slot)| Held {
                thread: thread.clone(),
                pid: slot.pid,
                exit: slot.exit,
            })
            .collect()
    }

    #[must_use]
    pub fn new_slot(&self, pid: u32, started: Option<String>, input: Input) -> Slot {
        Slot {
            pid,
            started,
            input,
            outbox: Outbox::new(self.limit),
            waiting: false,
            exit: None,
        }
    }
}

/// Runs the holder in `dir` until it gets `SIGTERM` or `Quit`. At the end it ends each program
/// it holds and the processes below them (THR-049).
///
/// # Errors
/// [`HolderError`] when the folder is not private, a different holder holds it, or the socket
/// cannot be made.
pub async fn hold(dir: &Path, limit: usize) -> Result<(), HolderError> {
    let _lock = lock(dir)?;
    record::end_left_over(dir).await;
    let listener = bind(dir)?;
    let shared = Arc::new(Mutex::new(Shared {
        threads: BTreeMap::new(),
        dir: dir.to_owned(),
        connection: None,
        connections: 0,
        done: BTreeMap::new(),
        done_order: VecDeque::new(),
        draining: false,
        limit: if limit == 0 { LIMIT } else { limit },
    }));
    let (quit, mut quitting) = mpsc::channel::<()>(1);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                if let Ok((stream, _)) = accepted {
                    tokio::spawn(serve(Arc::clone(&shared), stream, quit.clone()));
                }
            }
            _ = terminate.recv() => break,
            _ = quitting.recv() => break,
        }
    }
    super::commands::end_all(&shared).await;
    let _ = std::fs::remove_file(dir.join(SOCKET));
    Ok(())
}

/// Makes the folder private if it is new and takes its lock: a second holder ends at once.
fn lock(dir: &Path) -> Result<std::fs::File, HolderError> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)?;
    }
    if !crate::config::private_file::folder_private(dir)? {
        return Err(HolderError::Exposed);
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK))?;
    std::fs::set_permissions(dir.join(LOCK), std::fs::Permissions::from_mode(0o600))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| HolderError::AlreadyRunning)?;
    Ok(lock)
}

/// Binds the socket file with mode 0600, in place of one an older holder left (THR-128).
fn bind(dir: &Path) -> Result<UnixListener, HolderError> {
    use std::os::unix::fs::PermissionsExt as _;
    let socket = dir.join(SOCKET);
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// One connection: the peer must be this user; a frame that is too long or does not read closes
/// the connection and harms no agent (H8).
async fn serve(shared: Arc<Mutex<Shared>>, stream: UnixStream, quit: mpsc::Sender<()>) {
    let same_user = stream
        .peer_cred()
        .is_ok_and(|peer| peer.uid() == rustix::process::getuid().as_raw());
    if !same_user {
        return;
    }
    let (reading, mut writing) = stream.into_split();
    let (sender, mut outgoing) = mpsc::unbounded_channel::<String>();
    let me = {
        let Ok(mut state) = shared.lock() else { return };
        state.connections += 1;
        let me = state.connections;
        state.connection = Some((me, sender));
        me
    };
    let writer = tokio::spawn(async move {
        while let Some(frame) = outgoing.recv().await {
            if writing.write_all(frame.as_bytes()).await.is_err() {
                break;
            }
        }
    });
    let mut reader = BufReader::new(reading).take(u64::MAX);
    let mut line = Vec::new();
    loop {
        line.clear();
        reader.set_limit(u64::try_from(MAX_FRAME).unwrap_or(u64::MAX) + 1);
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Some(command) = wire::parse::<Command>(line.trim_ascii_end()) else {
            break;
        };
        let ends = matches!(command.action, Action::Quit);
        let outcome = super::commands::run(&shared, command).await;
        if ends && outcome == Outcome::Ok {
            let _ = quit.send(()).await;
        }
    }
    if let Ok(mut state) = shared.lock()
        && state.connection.as_ref().is_some_and(|(id, _)| *id == me)
    {
        state.connection = None;
    }
    writer.abort();
}
