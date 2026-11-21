//! The daemon side of the socket of the holder: it starts the holder when none answers, checks
//! that the socket file is of this user before it connects (THR-128), sends commands with IDs
//! and gives the events of the holder in order.

use std::io;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::mpsc;

use super::holder::SOCKET;
use super::wire::{self, Action, Command, Event, MAX_FRAME};

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("the socket of the holder: {0}")]
    Io(#[from] io::Error),
    #[error("the socket of the holder is not a socket of this user with mode 0600")]
    NotOurs,
    #[error("the holder did not answer in time")]
    NoHolder,
}

/// A connection to the holder.
pub struct HolderLink {
    writer: OwnedWriteHalf,
    next: u64,
    /// The events of the holder, in the order it sent them.
    pub events: mpsc::Receiver<Event>,
}

impl HolderLink {
    /// Connects to the holder in `dir`, after a check of the owner and the mode of the socket
    /// file, and sends `Hello`. The IDs of the commands start at the microseconds of the clock,
    /// so that a daemon that starts again never uses an ID of the one before it (H10).
    ///
    /// # Errors
    /// [`LinkError::NotOurs`] for a socket of a different user or mode; an I/O error when no
    /// holder listens.
    pub async fn connect(dir: &Path) -> Result<Self, LinkError> {
        let socket = dir.join(SOCKET);
        let metadata = std::fs::symlink_metadata(&socket)?;
        let ours = metadata.uid() == rustix::process::getuid().as_raw()
            && metadata.permissions().mode().trailing_zeros() >= 6
            && std::os::unix::fs::FileTypeExt::is_socket(&metadata.file_type());
        if !ours {
            return Err(LinkError::NotOurs);
        }
        let stream = UnixStream::connect(&socket).await?;
        let (reading, writer) = stream.into_split();
        let (sender, events) = mpsc::channel(1024);
        tokio::spawn(async move {
            // The reader holds at most one frame and its line end in memory (H8).
            let limit = u64::try_from(MAX_FRAME)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            let mut lines = BufReader::new(reading).take(limit);
            let mut line = Vec::new();
            loop {
                line.clear();
                lines.set_limit(limit);
                match lines.read_until(b'\n', &mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) if !line.ends_with(b"\n") => break,
                    Ok(_) => {}
                }
                let Some(event) = wire::parse::<Event>(line.trim_ascii_end()) else {
                    break;
                };
                if sender.send(event).await.is_err() {
                    break;
                }
            }
        });
        let first = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |since| {
                u64::try_from(since.as_micros()).unwrap_or(u64::MAX / 2)
            });
        let mut link = Self {
            writer,
            next: first,
            events,
        };
        link.send(Action::Hello {
            version: wire::VERSION,
        })
        .await?;
        Ok(link)
    }

    /// Sends one command and gives its ID.
    ///
    /// # Errors
    /// The I/O error when the holder is gone.
    pub async fn send(&mut self, action: Action) -> io::Result<u64> {
        let command = self.prepare(action);
        self.resend(&command).await?;
        Ok(command.id)
    }

    /// Gives `action` the next ID, to be sent with [`Self::resend`].
    pub fn prepare(&mut self, action: Action) -> Command {
        let id = self.next;
        self.next += 1;
        Command { id, action }
    }

    /// The ID that the next command takes.
    #[must_use]
    pub const fn next_id(&self) -> u64 {
        self.next
    }

    /// Goes on from the IDs of a link before this one, so that a clock that went back never
    /// gives an ID that the holder already answered (H10).
    pub fn go_on_from(&mut self, next: u64) {
        self.next = self.next.max(next);
    }

    /// Sends a command again with its own ID; the holder does it one time (H10).
    ///
    /// # Errors
    /// The I/O error when the holder is gone.
    pub async fn resend(&mut self, command: &Command) -> io::Result<()> {
        self.writer.write_all(wire::frame(command).as_bytes()).await
    }
}

/// Starts `program holder <dir>` with an empty environment, no open input or output, and a
/// process group of its own, so that the end of the daemon does not end it (THR-113). The
/// holder takes the lock of the folder; a second one ends at once.
///
/// # Errors
/// The I/O error when the program cannot start.
pub fn start_holder(program: &Path, dir: &Path) -> io::Result<()> {
    let mut child = std::process::Command::new(program)
        .arg("holder")
        .arg(dir)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    // The daemon reaps the holder when it ends, so that it leaves no zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Connects to the holder of `dir`, and starts one first when none answers.
///
/// # Errors
/// [`LinkError::NoHolder`] when no holder answers within a few seconds of its start.
pub async fn reach(program: &Path, dir: &Path) -> Result<HolderLink, LinkError> {
    if let Ok(link) = HolderLink::connect(dir).await {
        return Ok(link);
    }
    start_holder(program, dir)?;
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(link) = HolderLink::connect(dir).await {
            return Ok(link);
        }
    }
    Err(LinkError::NoHolder)
}

/// The folder of the holder below the folder of the daemon.
#[must_use]
pub fn folder(daemon_dir: &Path) -> PathBuf {
    daemon_dir.join("threads")
}
