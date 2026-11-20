//! The guard outlives a crashed app; closing its private stdin restores the original plan.
use super::journal::{self, Journal};
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    os::windows::process::CommandExt as _,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

const ARGUMENT: &str = "--anima-lid-guard";
const WAIT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct LidHold {
    child: Child,
    input: Option<ChildStdin>,
}
impl LidHold {
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}
impl Drop for LidHold {
    fn drop(&mut self) {
        self.input.take();
        // Never kill the guard: it must be allowed to restore even after a timeout.
        let end = std::time::Instant::now() + WAIT;
        while self.alive() && std::time::Instant::now() < end {
            thread::sleep(Duration::from_millis(20));
        }
    }
}

pub fn start(marker: &Path) -> io::Result<LidHold> {
    if !anima_windows::power::scheme::available() {
        return Err(io::ErrorKind::Unsupported.into());
    }
    let mut child = Command::new(std::env::current_exe()?)
        .arg(ARGUMENT)
        .arg(marker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()?;
    let input = child.stdin.take();
    let output = child.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
    let hold = LidHold { child, input };
    let (send, receive) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut line = String::new();
        let result = io::BufReader::new(output).take(32).read_line(&mut line);
        let _ = send.send(result.is_ok() && line == "ready\n");
    });
    if receive.recv_timeout(WAIT) != Ok(true) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(hold)
}

struct Restoration {
    journal: Journal,
    marker: PathBuf,
}
impl Drop for Restoration {
    fn drop(&mut self) {
        if self.journal.restore().is_ok() {
            let _ = fs::remove_file(&self.marker);
        }
    }
}

fn run(marker: &Path) -> io::Result<()> {
    if !anima_windows::power::scheme::available() {
        return Err(io::ErrorKind::Unsupported.into());
    }
    let restore = Restoration {
        journal: Journal::start(marker)?,
        marker: marker.to_owned(),
    };
    restore.journal.activate()?;
    let mut output = io::stdout().lock();
    output.write_all(b"ready\n")?;
    output.flush()?;
    let mut byte = [0];
    // Data or EOF both mean release. An app crash closes the writer automatically.
    let _ = io::stdin().read(&mut byte)?;
    drop(restore);
    Ok(())
}

pub fn entry() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(ARGUMENT)) {
        return false;
    }
    if let Some(marker) = args.next().filter(|_| args.next().is_none()) {
        let _ = run(Path::new(&marker));
    }
    true
}

pub fn restore(marker: &Path) -> io::Result<()> {
    journal::restore(marker)
}
