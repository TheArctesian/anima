//! One server process (Sections 2.1 and 9): started by the absolute path of its program, with
//! an empty environment and the fixed list, the owned flags, its own process group, and its
//! output in the ring. Stopped by a request to end, at most 10 seconds, then the end of the
//! whole group.

use std::io;
use std::net::TcpListener;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use rustix::process::{Pid, Signal};
use tokio::process::{Child, Command};

use super::ring::Ring;

/// The range of ports of the daemon configuration (default 41000 to 41999).
pub const PORTS: RangeInclusive<u16> = 41_000..=41_999;

/// What one start needs. Each word comes from the code of the daemon.
pub struct Launch {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub temporary: PathBuf,
    pub ring: PathBuf,
}

/// A running process.
pub struct Running {
    pub child: Child,
    /// The process group of the tree: the process id of the server.
    pub group: i32,
}

/// The first port of `range` that is free on loopback now. The listener check after the start
/// confirms that the server, not another program, holds it.
#[must_use]
pub fn free_port(range: RangeInclusive<u16>) -> Option<u16> {
    range
        .into_iter()
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
}

/// Empties the temporary folder of the process (no link followed) and makes it again, 0700.
///
/// # Errors
/// An I/O error of the folder.
pub fn fresh_temporary(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    super::folders::private_folder(path)
}

/// Starts the process of `launch`.
///
/// # Errors
/// An I/O error of the folder, the ring or the start.
pub fn spawn(launch: &Launch) -> io::Result<Running> {
    fresh_temporary(&launch.temporary)?;
    let ring = Ring::create(&launch.ring)?;
    let mut child = Command::new(&launch.program)
        .args(&launch.arguments)
        .env_clear()
        .envs(launch.environment.iter().map(|(name, value)| (name, value)))
        .current_dir(&launch.temporary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()?;
    let group = child
        .id()
        .and_then(|id| i32::try_from(id).ok())
        .ok_or_else(|| io::Error::other("the server has no process id"))?;
    if let Some(output) = child.stdout.take() {
        ring.follow(output);
    }
    if let Some(output) = child.stderr.take() {
        ring.follow(output);
    }
    Ok(Running { child, group })
}

fn signal_group(group: i32, signal: Signal) {
    if let Some(pid) = Pid::from_raw(group) {
        let _ = rustix::process::kill_process_group(pid, signal);
    }
}

/// Ends the whole tree at once.
pub async fn kill(running: &mut Running) {
    signal_group(running.group, Signal::KILL);
    let _ = running.child.wait().await;
}

/// Asks the tree to end, waits at most 10 seconds, then ends it.
pub async fn stop(running: &mut Running) {
    signal_group(running.group, Signal::TERM);
    if tokio::time::timeout(Duration::from_secs(10), running.child.wait())
        .await
        .is_err()
    {
        kill(running).await;
    }
}
