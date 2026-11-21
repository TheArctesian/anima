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
    /// The program ended and was reaped: its ID may belong to a different process now.
    pub reaped: bool,
    pub exit: Option<Exit>,
}

/// What the tasks of the holder share.
pub struct Shared {
    pub threads: BTreeMap<ThreadKey, Slot>,
    pub dir: PathBuf,
    connection: Option<(u64, mpsc::UnboundedSender<String>)>,
    pub(super) done: BTreeMap<u64, Outcome>,
    done_order: VecDeque<u64>,
    /// The daemon speaks a different version: no new start (THR-113).
    pub draining: bool,
    /// The holder ends: no new start, so that no program outlives it (THR-049).
    pub closing: bool,
    pub limit: usize,
}

impl Shared {
    /// Gives an event to the daemon if one is connected; else the outbox keeps the lines. A
    /// connection that closed is the same as none: its lines come again at the next `Hello`.
    pub fn send(&self, event: &Event) {
        if let Some((_, sender)) = &self.connection {
            let _ = sender.send(wire::frame(event));
        }
    }

    /// Whether the connection `me` is the one that the holder serves now.
    fn serves(&self, me: u64) -> bool {
        self.connection.as_ref().is_some_and(|(id, _)| *id == me)
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
        let written = record::write(
            &self.dir,
            &Record {
                version: wire::VERSION,
                threads: running.collect(),
            },
        );
        if let Err(error) = written {
            tracing::warn!(%error, "the record of the holder was not written");
        }
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
            reaped: false,
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
        done: BTreeMap::new(),
        done_order: VecDeque::new(),
        draining: false,
        closing: false,
        limit: if limit == 0 { LIMIT } else { limit },
    }));
    // One command at a time, from any connection: the check that a command was done and the
    // command itself are one step, so a command sent again on a new connection while the first
    // runs is still done one time (H10).
    let turn = Arc::new(tokio::sync::Mutex::new(()));
    accept(&listener, &shared, &turn).await?;
    let _only = turn.lock().await;
    if let Ok(mut state) = shared.lock() {
        state.closing = true;
        state.connection = None;
    }
    super::commands::end_all(&shared).await;
    let _ = std::fs::remove_file(dir.join(SOCKET));
    Ok(())
}

/// Accepts connections until `SIGTERM` or `Quit`. The number of a connection is given in the
/// order of the accepts, so the newest connection is the one accepted last, not the one whose
/// task ran last (H10).
async fn accept(
    listener: &UnixListener,
    shared: &Arc<Mutex<Shared>>,
    turn: &Arc<tokio::sync::Mutex<()>>,
) -> io::Result<()> {
    let (quit, mut quitting) = mpsc::channel::<()>(1);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut count: u64 = 0;
    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    count += 1;
                    let connection = Connection {
                        me: count,
                        turn: Arc::clone(turn),
                        quit: quit.clone(),
                    };
                    tokio::spawn(serve(Arc::clone(shared), stream, connection));
                }
                Err(error) => {
                    // For example no free descriptor: wait, so that the loop does not spin.
                    tracing::warn!(%error, "the holder could not accept a connection");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            },
            _ = terminate.recv() => return Ok(()),
            _ = quitting.recv() => return Ok(()),
        }
    }
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

/// What one connection of the holder needs besides the shared state.
struct Connection {
    /// The number of the connection, in the order of the accepts.
    me: u64,
    /// The lock that lets one command run at a time.
    turn: Arc<tokio::sync::Mutex<()>>,
    quit: mpsc::Sender<()>,
}

/// One connection: the peer must be this user; a frame that is too long or does not read closes
/// the connection and harms no agent (H8). It takes the place of an older connection, never of a
/// newer one, and it stops at once when a newer one takes its place.
async fn serve(shared: Arc<Mutex<Shared>>, stream: UnixStream, connection: Connection) {
    let Connection { me, turn, quit } = connection;
    let same_user = stream
        .peer_cred()
        .is_ok_and(|peer| peer.uid() == rustix::process::getuid().as_raw());
    if !same_user {
        return;
    }
    let (reading, mut writing) = stream.into_split();
    let (sender, mut outgoing) = mpsc::unbounded_channel::<String>();
    {
        let _one = turn.lock().await;
        let Ok(mut state) = shared.lock() else { return };
        if state.closing || state.connection.as_ref().is_some_and(|(id, _)| *id > me) {
            return;
        }
        state.connection = Some((me, sender));
    }
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
        let one = turn.lock().await;
        if !shared.lock().is_ok_and(|state| state.serves(me)) {
            break;
        }
        let outcome = super::commands::run(&shared, command).await;
        drop(one);
        if ends && outcome == Outcome::Ok {
            let _ = quit.send(()).await;
        }
    }
    if let Ok(mut state) = shared.lock()
        && state.serves(me)
    {
        state.connection = None;
    }
    writer.abort();
}
