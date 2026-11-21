//! The local socket of the thread runner of this computer (TH12, THR-128): the `anima` command
//! is a client of the daemon here, not of the hub. Nothing goes to the network.

use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::PathBuf;

use agent_threads::control::{self, Action, Call, Outcome, Refusal, Reply};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use crate::failure::{Failure, Hint};

/// A connection to the runner.
pub struct RunnerLink {
    lines: Lines<BufReader<OwnedReadHalf>>,
    write: OwnedWriteHalf,
    next: u64,
}

fn not_running() -> Failure {
    Failure::NotFound {
        message: "The thread runner of this computer is not running.".to_owned(),
        hint: Some(Hint::run("anima-daemon run")),
    }
}

/// The socket file of the runner, as the daemon finds its folder.
fn socket() -> Option<PathBuf> {
    let folder = control::folder(|name| std::env::var(name).ok(), cfg!(target_os = "macos"))?;
    Some(folder.join(control::SOCKET))
}

impl RunnerLink {
    /// Connects, after a check that the socket file is a socket of this user that no one else
    /// may use (THR-128).
    ///
    /// # Errors
    /// [`Failure::NotFound`] when no runner listens; [`Failure::Forbidden`] for a socket of a
    /// different user or mode.
    pub async fn connect() -> Result<Self, Failure> {
        let socket = socket().ok_or_else(not_running)?;
        let meta = std::fs::symlink_metadata(&socket).map_err(|_| not_running())?;
        let ours = meta.uid() == rustix::process::getuid().as_raw()
            && meta.permissions().mode().trailing_zeros() >= 6
            && meta.file_type().is_socket();
        if !ours {
            return Err(Failure::Forbidden {
                message: format!("{} is not a private socket of this user.", socket.display()),
                hint: None,
            });
        }
        let stream = UnixStream::connect(&socket)
            .await
            .map_err(|_| not_running())?;
        let (read, write) = stream.into_split();
        Ok(Self {
            lines: BufReader::new(read).lines(),
            write,
            next: 0,
        })
    }

    /// Sends a call and gives its ID.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when the runner is gone.
    pub async fn send(&mut self, action: Action) -> Result<u64, Failure> {
        self.next += 1;
        let call = Call {
            id: self.next,
            action,
        };
        let mut text = serde_json::to_string(&call).map_err(|error| Failure::Write {
            reason: error.to_string(),
        })?;
        text.push('\n');
        self.write
            .write_all(text.as_bytes())
            .await
            .map_err(|_| not_running())?;
        Ok(self.next)
    }

    /// The next frame of the runner; `None` when it closed the connection.
    ///
    /// # Errors
    /// [`Failure::Input`] for a frame that does not read.
    pub async fn next(&mut self) -> Result<Option<Reply>, Failure> {
        let Some(line) = self.lines.next_line().await.map_err(|_| not_running())? else {
            return Ok(None);
        };
        serde_json::from_str(&line)
            .map(Some)
            .map_err(|error| Failure::Input {
                what: "a frame of the thread runner".to_owned(),
                reason: error.to_string(),
            })
    }

    /// Sends a call and waits for its answer.
    ///
    /// # Errors
    /// As [`Self::send`] and [`Self::next`].
    pub async fn call(&mut self, action: Action) -> Result<Outcome, Failure> {
        let id = self.send(action).await?;
        loop {
            match self.next().await?.ok_or_else(not_running)? {
                Reply::Done { id: done, result } if done == id => return Ok(result),
                _ => {}
            }
        }
    }
}

/// A refusal of the runner as a failure of the command, in words for the person.
#[must_use]
pub fn refusal(why: &Refusal) -> Failure {
    let (message, hint) = match why {
        Refusal::UnknownThread => (
            "No thread of this computer has that ID.",
            Some(Hint::run("anima agents running")),
        ),
        Refusal::FolderNotPermitted => (
            "That folder is not an absolute folder of this computer.",
            None,
        ),
        Refusal::ProgramNotFound => (
            "That program is not on the PATH of your login, or that path is not a copy the runner finds.",
            None,
        ),
        Refusal::ProgramDoesNotAnswer { said, .. } | Refusal::HolderOlder { said } => {
            return Failure::Forbidden {
                message: said.clone(),
                hint: None,
            };
        }
        Refusal::BadValue => (
            "A value is not permitted (a model name takes letters, digits, '.', '_', '-', '[' and ']').",
            None,
        ),
        Refusal::RequestGone => (
            "That request is answered, closed, or its program ended.",
            None,
        ),
        Refusal::AnswerDoesNotFit => ("That answer does not fit the request.", None),
        Refusal::NotRunning => ("The program of that thread is not running.", None),
        Refusal::Holder => ("The holder of the programs did not take the command.", None),
        Refusal::FromAnAgent => (
            "Only the person can answer a request, raise the limit or start a thread: this call came from a process of an agent.",
            None,
        ),
        Refusal::OverLimit { .. } => (
            control::OVER_LIMIT,
            Some(Hint::run("anima agents raise-limit")),
        ),
    };
    Failure::Forbidden {
        message: message.to_owned(),
        hint,
    }
}
