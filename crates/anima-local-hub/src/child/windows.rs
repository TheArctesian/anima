//! Windows children have a private stop pipe and a job owning all descendants.
use super::{Child, ChildError, Spec, Stop, Stopped, copy_lines, wait};
use crate::log_file::LogFile;
use anima_windows::job::Job;
use std::{io, os::windows::io::OwnedHandle, process::Stdio, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    process::Command,
    sync::{mpsc, watch},
};

impl Stop {
    /// Add the per-user control socket after the run lock has been acquired.
    pub(crate) fn with_control(mut self, path: &std::path::Path) -> io::Result<Self> {
        let listener = anima_windows::socket::Listener::bind(path)?;
        let (ask, stop) = Self::new();
        tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    () = self.asked() => break,
                    () = ask.closed() => return,
                    accepted = listener.accept() => accepted,
                };
                let Ok(stream) = accepted else {
                    break;
                };
                if stop_request(stream).await {
                    break;
                }
            }
            ask.send_replace(true);
            ask.closed().await;
        });
        Ok(stop)
    }
    /// Buffer console or managed-parent shutdown requests.
    /// # Errors
    /// Windows cannot install the signal listeners.
    pub fn on_signals() -> io::Result<Self> {
        let mut requested = anima_windows::shutdown::Requested::new()?;
        let (ask, stop) = Self::new();
        tokio::spawn(async move {
            requested.wait().await;
            ask.send_replace(true);
            ask.closed().await;
        });
        Ok(stop)
    }
}

impl Child {
    /// Start a hidden member inside an owned job before allowing its first instruction.
    /// # Errors
    /// The command, output pipe or job cannot be created.
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
        let (output, input) = std::io::pipe().map_err(failed)?;
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .env_clear()
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(input.try_clone().map_err(failed)?)
            .stderr(input)
            .creation_flags(anima_windows::job::SUSPENDED)
            .kill_on_drop(true);
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut child = command.spawn().map_err(failed)?;
        let pid = child
            .id()
            .ok_or_else(|| failed(io::ErrorKind::Other.into()))?;
        let job = Job::start(pid).map_err(failed)?;
        let input = child.stdin.take();
        let output = tokio::fs::File::from_std(std::fs::File::from(OwnedHandle::from(output)));
        let (line, lines) = mpsc::unbounded_channel();
        tokio::spawn(copy_lines(output, log, line));
        let (report, exit) = watch::channel(None);
        tokio::spawn(async move {
            let status = child.wait().await.unwrap_or_default();
            report.send_replace(Some(status));
            let _ = ended.send(name);
        });
        Ok(Self {
            name,
            input,
            job: Some(job),
            exit,
            lines: Some(lines),
        })
    }

    /// Request graceful shutdown on the private pipe, then kill the entire job at the limit.
    /// # Errors
    /// A live member's private stop pipe cannot be written.
    pub async fn stop(mut self, limit: Duration) -> Result<Stopped, ChildError> {
        if let Some(status) = self.ended() {
            return Ok(Stopped::Ended(status));
        }
        if let Some(mut input) = self.input.take() {
            // A member may exit while its input is being written; the bounded wait decides.
            let _ = input.write_all(b"stop\n").await;
        }
        if let Ok(status) = self.end_within(limit).await {
            return Ok(Stopped::Ended(status));
        }
        self.job.take();
        wait(&mut self.exit).await;
        Ok(Stopped::Killed)
    }
}

async fn stop_request(mut stream: anima_windows::socket::Stream) -> bool {
    use tokio::io::AsyncReadExt;
    let mut line = [0; 5];
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut line))
        .await
        .is_ok_and(|read| read.is_ok())
        && &line == b"stop\n"
}
