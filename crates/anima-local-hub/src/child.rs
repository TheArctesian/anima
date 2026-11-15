//! One member of the group as a child process: a sealed environment, its output in its own
//! log file, a line that tells it is ready, and a stop with a limit.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};

use crate::log_file::LogFile;

#[cfg(test)]
mod tests;

/// The most bytes of one output line that go to the log.
const LINE_LIMIT: usize = 16 * 1024;

/// What to run. The child gets `env` and nothing else of this program's environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub name: &'static str,
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(&'static str, OsString)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChildError {
    #[error("{name} cannot be started ({kind})")]
    Start {
        name: &'static str,
        kind: io::ErrorKind,
    },
    #[error("{name} ended ({status})")]
    Ended {
        name: &'static str,
        status: ExitStatus,
    },
    #[error("{name} was not ready in {} s", limit.as_secs())]
    NotReady { name: &'static str, limit: Duration },
    #[error("a signal cannot be sent to {name}")]
    Signal { name: &'static str },
}

impl ChildError {
    /// The exit code of a member that ended by itself.
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Ended { status, .. } => status.code(),
            _ => None,
        }
    }
}

/// The request to stop the group: SIGTERM or SIGINT in the program, a handle in a test.
#[derive(Debug, Clone)]
pub struct Stop(watch::Receiver<bool>);

impl Stop {
    /// A request that `ask` makes.
    #[must_use]
    pub fn new() -> (watch::Sender<bool>, Self) {
        let (ask, asked) = watch::channel(false);
        (ask, Self(asked))
    }

    /// A request that SIGTERM or SIGINT makes. A signal that comes before anybody waits is kept.
    ///
    /// # Errors
    /// When the signals cannot be watched.
    pub fn on_signals() -> io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())?;
        let mut interrupt = signal(SignalKind::interrupt())?;
        let (ask, stop) = Self::new();
        tokio::spawn(async move {
            tokio::select! {
                _ = terminate.recv() => {}
                _ = interrupt.recv() => {}
            }
            ask.send_replace(true);
            // Kept: a closed channel would read as "never asked" to a late clone.
            ask.closed().await;
        });
        Ok(stop)
    }

    #[must_use]
    pub fn is_asked(&self) -> bool {
        *self.0.borrow()
    }

    /// Returns when the stop is asked for. Never returns when the asking side is gone.
    pub async fn asked(&mut self) {
        if self.0.wait_for(|asked| *asked).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// How a member ended when it was asked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// It ended by itself, before or after SIGTERM.
    Ended(ExitStatus),
    /// It did not end in the limit and was killed.
    Killed,
}

/// A running member.
#[derive(Debug)]
pub struct Child {
    name: &'static str,
    process: Pid,
    exit: watch::Receiver<Option<ExitStatus>>,
    lines: Option<mpsc::UnboundedReceiver<String>>,
}

impl Child {
    /// Starts `spec`. `ended` gets the member's name when it ends.
    ///
    /// The member stays in this program's process group. When this program is killed, launchd
    /// kills what is left of its process group and systemd what is left of its control group,
    /// so no member outlives a unit's run and holds the database or the hub's port. The price:
    /// Ctrl-C in a terminal reaches every member at once, not in order.
    ///
    /// # Errors
    /// [`ChildError::Start`] when the program cannot be run.
    pub fn start(
        spec: &Spec,
        log: LogFile,
        ended: mpsc::UnboundedSender<&'static str>,
    ) -> Result<Self, ChildError> {
        let name = spec.name;
        let failed = |error: io::Error| ChildError::Start {
            name,
            kind: error.kind(),
        };
        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .env_clear()
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(failed)?;
        let process = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .map(Pid::from_raw)
            .ok_or_else(|| failed(io::ErrorKind::Other.into()))?;
        let log = Arc::new(Mutex::new(log));
        let (line, lines) = mpsc::unbounded_channel();
        if let Some(output) = child.stdout.take() {
            tokio::spawn(copy_lines(output, Arc::clone(&log), line.clone()));
        }
        if let Some(output) = child.stderr.take() {
            tokio::spawn(copy_lines(output, log, line));
        }
        let (report, exit) = watch::channel(None);
        tokio::spawn(async move {
            // An error here means the child cannot be waited for; the group treats that as ended.
            let status = child.wait().await.unwrap_or_default();
            report.send_replace(Some(status));
            let _ = ended.send(name);
        });
        Ok(Self {
            name,
            process,
            exit,
            lines: Some(lines),
        })
    }

    /// The exit status when the member has ended.
    #[must_use]
    pub fn ended(&self) -> Option<ExitStatus> {
        *self.exit.borrow()
    }

    /// Reads the member's output lines until `pick` takes one.
    ///
    /// # Errors
    /// [`ChildError::Ended`] when the member ends first, [`ChildError::NotReady`] after `limit`.
    pub async fn line_where<T>(
        &mut self,
        limit: Duration,
        mut pick: impl FnMut(&str) -> Option<T>,
    ) -> Result<T, ChildError> {
        let name = self.name;
        let read = async {
            while let Some(lines) = self.lines.as_mut() {
                // `None`: both outputs are closed. The member ends, or writes no more.
                let Some(line) = lines.recv().await else {
                    self.lines = None;
                    break;
                };
                if let Some(picked) = pick(&line) {
                    return Ok(picked);
                }
            }
            let status = wait(&mut self.exit).await;
            Err(ChildError::Ended { name, status })
        };
        tokio::time::timeout(limit, read)
            .await
            .unwrap_or(Err(ChildError::NotReady { name, limit }))
    }

    /// Stops reading the output for a line; it still goes to the log.
    pub fn stop_reading(&mut self) {
        self.lines = None;
    }

    /// Waits for the member to end by itself.
    ///
    /// # Errors
    /// [`ChildError::NotReady`] after `limit`.
    pub async fn end_within(&mut self, limit: Duration) -> Result<ExitStatus, ChildError> {
        tokio::time::timeout(limit, wait(&mut self.exit))
            .await
            .map_err(|_| ChildError::NotReady {
                name: self.name,
                limit,
            })
    }

    /// SIGTERM, then `limit` to end, then SIGKILL. A member's own children are its to stop; the
    /// database program stops a PostgreSQL server that a killed copy left at its next start.
    ///
    /// # Errors
    /// [`ChildError::Signal`] when a live member cannot be signalled.
    pub async fn stop(mut self, limit: Duration) -> Result<Stopped, ChildError> {
        if let Some(status) = self.ended() {
            return Ok(Stopped::Ended(status));
        }
        self.signal(Signal::SIGTERM)?;
        if let Ok(status) = self.end_within(limit).await {
            return Ok(Stopped::Ended(status));
        }
        self.signal(Signal::SIGKILL)?;
        wait(&mut self.exit).await;
        Ok(Stopped::Killed)
    }

    /// A process that ended between the look and the signal is not an error.
    fn signal(&self, signal: Signal) -> Result<(), ChildError> {
        if self.ended().is_some() {
            return Ok(());
        }
        match kill(self.process, signal) {
            Ok(()) | Err(nix::errno::Errno::ESRCH) => Ok(()),
            Err(_) => Err(ChildError::Signal { name: self.name }),
        }
    }
}

async fn wait(exit: &mut watch::Receiver<Option<ExitStatus>>) -> ExitStatus {
    match exit.wait_for(Option::is_some).await {
        Ok(status) => status.unwrap_or_default(),
        // The task that waits for the child is gone, so the child is.
        Err(_) => ExitStatus::default(),
    }
}

/// Copies each line of `output` to the log and to whoever reads for a ready line. It reads to
/// the end also when nobody takes the lines: a full pipe would stop the member.
async fn copy_lines(
    output: impl AsyncRead + Unpin,
    log: Arc<Mutex<LogFile>>,
    line: mpsc::UnboundedSender<String>,
) {
    let mut output = BufReader::new(output);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match output.read_until(b'\n', &mut bytes).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(LINE_LIMIT)]);
        let text = text.trim_end_matches(['\n', '\r']);
        // A log that cannot be written must not stop the member.
        let _ = log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .line(text);
        let _ = line.send(text.to_owned());
    }
}
