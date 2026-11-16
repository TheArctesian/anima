//! Runs a program the application carries, once: an absolute program and fixed arguments, no
//! shell, an environment that holds only the given variables, and a limit after which it is
//! killed. [`run`] gives the program's first error line for the app's log; [`exchange`] gives
//! the program its input on standard input and never makes text of what it said, for the runs
//! that carry a secret (LOG-007).

use std::{
    ffi::OsString,
    fmt,
    io::{ErrorKind, Read, Write},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use zeroize::Zeroizing;

const POLL: Duration = Duration::from_millis(20);
/// The most the program's answer or its error message can be.
const MOST_OUTPUT: u64 = 64 * 1024;

/// One run: what runs, with what, and for how long at most.
#[derive(Debug, Clone, Copy)]
pub struct Invocation<'a> {
    pub program: &'a Path,
    pub args: &'a [&'a OsString],
    pub env: &'a [(&'static str, OsString)],
    pub limit: Duration,
}

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

/// What a program said on standard error. It may hold a secret, so it has no `Display`, and its
/// `Debug` gives only its length.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Said(Zeroizing<Vec<u8>>);

impl Said {
    /// What the program said, as text, to be read and not to be logged.
    pub fn text(&self) -> Zeroizing<String> {
        Zeroizing::new(String::from_utf8_lossy(&self.0).into_owned())
    }
}

impl fmt::Debug for Said {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "Said({} bytes)", self.0.len())
    }
}

/// Why an [`exchange`] did not succeed. Its text names no input, no output and no path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExchangeError {
    #[error("the program could not start")]
    CouldNotStart,
    #[error("the program could not be given its input")]
    Input,
    #[error("the program ran past its limit")]
    TimedOut,
    #[error("the program's exit status could not be read")]
    Unreadable,
    /// The exit code (none when a signal ended it) and what it said on standard error.
    #[error("the program exited with a failure")]
    Exited { code: Option<i32>, said: Said },
}

/// What the program printed on standard output.
pub fn run(invocation: &Invocation<'_>) -> Result<Vec<u8>, ProgramError> {
    let mut child = start(invocation, Stdio::null()).map_err(|()| ProgramError::CouldNotStart)?;
    // Read while the program runs: one that fills a pipe would never end.
    let printed = child.stdout.take().map(read_in_thread);
    let complained = child.stderr.take().map(read_in_thread);
    let status = match wait(&mut child, invocation.limit) {
        Ok(status) => status,
        Err(Ended::TimedOut) => return Err(ProgramError::TimedOut),
        Err(Ended::Unreadable) => return Err(ProgramError::Unreadable),
    };
    if !status.success() {
        let complaint = String::from_utf8_lossy(&joined(complained)).into_owned();
        let first = complaint.lines().next().unwrap_or("it gave no cause");
        return Err(ProgramError::Failed(first.to_owned()));
    }
    Ok(joined(printed).to_vec())
}

/// Writes `input` to the program's standard input, closes it, and returns what the program
/// printed on standard output. With `None` the program gets no input at all.
pub fn exchange(
    invocation: &Invocation<'_>,
    input: Option<&[u8]>,
) -> Result<Zeroizing<Vec<u8>>, ExchangeError> {
    let stdin = if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    let mut child = start(invocation, stdin).map_err(|()| ExchangeError::CouldNotStart)?;
    let printed = child.stdout.take().map(read_in_thread);
    let complained = child.stderr.take().map(read_in_thread);
    if let Err(error) = hand_over(&mut child, input) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = match wait(&mut child, invocation.limit) {
        Ok(status) => status,
        Err(Ended::TimedOut) => return Err(ExchangeError::TimedOut),
        Err(Ended::Unreadable) => return Err(ExchangeError::Unreadable),
    };
    if !status.success() {
        return Err(ExchangeError::Exited {
            code: status.code(),
            said: Said(joined(complained)),
        });
    }
    Ok(joined(printed))
}

fn start(invocation: &Invocation<'_>, stdin: Stdio) -> Result<Child, ()> {
    Command::new(invocation.program)
        .args(invocation.args)
        .env_clear()
        .envs(invocation.env.iter().map(|(name, value)| (name, value)))
        .current_dir("/")
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(drop)
}

/// Writes `input` and closes standard input. A child that exits before it reads leaves a broken
/// pipe; that is not an error here, as its exit status tells what happened.
fn hand_over(child: &mut Child, input: Option<&[u8]>) -> Result<(), ExchangeError> {
    let Some(input) = input else {
        return Ok(());
    };
    let mut stdin = child.stdin.take().ok_or(ExchangeError::Input)?;
    match stdin.write_all(input) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(_) => Err(ExchangeError::Input),
    }
}

enum Ended {
    TimedOut,
    Unreadable,
}

/// The child's exit status; at the limit the child is killed and reaped.
fn wait(child: &mut Child, limit: Duration) -> Result<ExitStatus, Ended> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Ended::TimedOut);
            }
            Ok(None) => thread::sleep(POLL),
            Err(_) => return Err(Ended::Unreadable),
        }
    }
}

fn joined(output: Option<thread::JoinHandle<Zeroizing<Vec<u8>>>>) -> Zeroizing<Vec<u8>> {
    output
        .and_then(|output| output.join().ok())
        .unwrap_or_default()
}

fn read_in_thread(output: impl Read + Send + 'static) -> thread::JoinHandle<Zeroizing<Vec<u8>>> {
    thread::spawn(move || {
        let mut all = Zeroizing::new(Vec::new());
        let _ = output.take(MOST_OUTPUT).read_to_end(&mut all);
        all
    })
}
