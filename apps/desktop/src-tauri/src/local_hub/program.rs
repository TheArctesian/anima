//! Runs `anima-local-hub` once: an absolute program and fixed arguments, no shell, an
//! environment that holds only the given variables, and a limit after which it is killed.

use std::{
    ffi::OsString,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(20);
/// The most the program's answer or its error message can be.
const MOST_OUTPUT: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProgramError {
    #[error("the program could not start")]
    CouldNotStart,
    /// The first line the program wrote to standard error. It names files, never a secret.
    #[error("{0}")]
    Failed(String),
    #[error("the program ran past its limit")]
    TimedOut,
    #[error("the program's exit status could not be read")]
    Unreadable,
}

/// What the program printed on standard output.
pub fn run(
    program: &Path,
    args: &[&OsString],
    env: &[(&'static str, OsString)],
    limit: Duration,
) -> Result<Vec<u8>, ProgramError> {
    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .envs(env.iter().map(|(name, value)| (name, value)))
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ProgramError::CouldNotStart)?;
    // Read while the program runs: one that fills a pipe would never end.
    let printed = child.stdout.take().map(read_in_thread);
    let complained = child.stderr.take().map(read_in_thread);
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProgramError::TimedOut);
            }
            Ok(None) => thread::sleep(POLL),
            Err(_) => return Err(ProgramError::Unreadable),
        }
    };
    let read = |output: Option<thread::JoinHandle<Vec<u8>>>| {
        output
            .and_then(|output| output.join().ok())
            .unwrap_or_default()
    };
    if !status.success() {
        let complaint = String::from_utf8_lossy(&read(complained)).into_owned();
        let first = complaint.lines().next().unwrap_or("it gave no cause");
        return Err(ProgramError::Failed(first.to_owned()));
    }
    Ok(read(printed))
}

fn read_in_thread(output: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut all = Vec::new();
        let _ = output.take(MOST_OUTPUT).read_to_end(&mut all);
        all
    })
}
