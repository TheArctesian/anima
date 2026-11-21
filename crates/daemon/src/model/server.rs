//! One server process (Sections 2.1 and 9): started by the absolute path of its program, with
//! an empty environment and the fixed list, the owned flags, in its tree ([`super::tree`]),
//! and its output in the ring. Stopped by a request to end, at most 10 seconds, then the end
//! of the whole tree.

use std::io;
use std::net::TcpListener;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use daemon_core::model_work::cause::{Cause, Containment};
use rustix::process::Signal;
use tokio::process::{Child, Command};
use zeroize::{Zeroize as _, Zeroizing};

use super::ring::Ring;
use super::tree::{self, Identity, Record};

/// The range of ports of the daemon configuration (default 41000 to 41999).
pub const PORTS: RangeInclusive<u16> = 41_000..=41_999;

/// What one start needs. Each word comes from the code of the daemon. The values of the
/// environment hold the API key, so they are wiped when the launch drops.
pub struct Launch {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
    /// The state folder and the path below it of the temporary folder of the process.
    pub state: PathBuf,
    pub temporary: PathBuf,
    pub ring: PathBuf,
    pub key: Zeroizing<String>,
    /// The folder of the server in the state folder; it holds the record of the tree.
    pub place: PathBuf,
    /// The server, which names its cgroup on Linux.
    pub server: uuid::Uuid,
    /// The daemon program, which is the launcher on Linux (R-NEW-containment).
    pub launcher: PathBuf,
}

impl Drop for Launch {
    fn drop(&mut self) {
        for (_, value) in &mut self.environment {
            value.zeroize();
        }
    }
}

/// A running server and its tree. When it drops, each process of the tree whose identity
/// holds is ended, and the cgroup with it, so no part of the tree outlives the record of the
/// daemon (a stop of the daemon, a drop of the node).
pub struct Running {
    pub child: Child,
    /// The main process of the server.
    pub leader: Identity,
    /// The processes of the tree at the last poll; the leader is one of them.
    pub known: Vec<Identity>,
    pub cgroup: Option<PathBuf>,
    record: PathBuf,
    /// The API key of this start, in memory only (Section 2.1).
    pub key: Zeroizing<String>,
    /// The tree was ended and waited for; it is not signalled again.
    ended: bool,
}

impl Drop for Running {
    fn drop(&mut self) {
        if !self.ended {
            self.end_now();
        }
    }
}

/// The parts of a running server that end its tree, apart from the handle of its child, so
/// the end runs on a blocking thread (it reads `ps` on macOS and waits for the cgroup).
#[derive(Clone)]
struct Tree {
    leader: Identity,
    known: Vec<Identity>,
    cgroup: Option<PathBuf>,
    record: PathBuf,
}

impl Tree {
    /// Sends `signal` to each known process whose identity holds.
    fn signal_all(&self, signal: Signal) {
        for identity in self.known.iter().chain([&self.leader]) {
            let _gone_or_sent = tree::signal(identity, signal);
        }
    }

    /// Ends the whole tree now: the cgroup, and each known process whose identity holds. On
    /// macOS the walk is read once more first, so a child made after the last poll is ended
    /// too.
    fn end(mut self) {
        #[cfg(not(target_os = "linux"))]
        if let Some(table) = super::tree::walk::table() {
            let roots: Vec<Identity> = self.known.iter().chain([&self.leader]).cloned().collect();
            for found in super::tree::walk::descendants(&table, &roots) {
                if !self.known.contains(&found) {
                    self.known.push(found);
                }
            }
        }
        let record = Record {
            known: self.known.clone(),
            cgroup: self.cgroup.clone(),
        };
        record.end_all();
        self.signal_all(Signal::KILL);
        let _ = std::fs::remove_file(&self.record);
    }
}

impl Running {
    /// The record of the tree now.
    fn record(&self) -> Record {
        Record {
            known: self.known.clone(),
            cgroup: self.cgroup.clone(),
        }
    }

    /// The step at which the launcher failed, when the main process ended with the exit code of
    /// a launcher step and the launcher wrote that code to its failure file (an exit code of
    /// the runtime alone is not taken as one of the launcher).
    #[must_use]
    pub fn launch_failure(&self, status: std::process::ExitStatus) -> Option<Containment> {
        let step = Containment::of_exit_code(status.code()?)?;
        let path = self.record.with_file_name(tree::LAUNCH_FAILURE);
        let written = tree::read_capped(&path, tree::LAUNCH_FAILURE_BYTES)?;
        let code = std::str::from_utf8(&written).ok()?.trim().parse().ok();
        (code == step.exit_code()).then_some(step)
    }

    fn tree(&self) -> Tree {
        Tree {
            leader: self.leader.clone(),
            known: self.known.clone(),
            cgroup: self.cgroup.clone(),
            record: self.record.clone(),
        }
    }

    /// Reads the processes of the tree, keeps them, and writes the record.
    ///
    /// # Errors
    /// An error of the read of the tree (the cgroup on Linux, the walk on macOS), or an empty
    /// tree while the main process still runs (it left the tree): both fail closed.
    pub async fn poll(&mut self) -> io::Result<Vec<Identity>> {
        #[cfg(target_os = "linux")]
        let members = {
            let cgroup = self
                .cgroup
                .clone()
                .ok_or_else(|| io::Error::other("no cgroup"))?;
            tokio::task::spawn_blocking(move || super::tree::linux::members(&cgroup))
                .await
                .map_err(io::Error::other)??
        };
        #[cfg(not(target_os = "linux"))]
        let members = {
            let roots: Vec<Identity> = self.known.iter().chain([&self.leader]).cloned().collect();
            let read = tokio::task::spawn_blocking(super::tree::walk::table);
            let table = tokio::time::timeout(Duration::from_secs(5), read)
                .await
                .map_err(|_| io::Error::other("ps did not end in 5 seconds"))?
                .map_err(io::Error::other)?
                .ok_or_else(|| io::Error::other("ps did not read"))?;
            super::tree::walk::descendants(&table, &roots)
        };
        let running = matches!(self.child.try_wait(), Ok(None));
        if running && members.is_empty() {
            return Err(io::Error::other("the main process runs outside its tree"));
        }
        // With a delegated cgroup a runtime can move itself into a different cgroup of the
        // daemon; a main process that runs but is not in its own cgroup has left it (T1).
        #[cfg(target_os = "linux")]
        if running && !members.contains(&self.leader) {
            return Err(io::Error::other(
                "the main process left the cgroup of its server",
            ));
        }
        self.known.clone_from(&members);
        self.record().save(&self.record);
        Ok(members)
    }

    /// Ends the whole tree on this thread: only for a drop, which cannot wait.
    fn end_now(&mut self) {
        self.tree().end();
    }

    /// Ends the whole tree on a blocking thread.
    async fn end_off_thread(&self) {
        let tree = self.tree();
        let _ = tokio::task::spawn_blocking(move || tree.end()).await;
    }
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
pub fn fresh_temporary(state: &Path, path: &Path) -> io::Result<()> {
    let relative = path
        .strip_prefix(state)
        .map_err(|_| io::Error::other("the temporary folder is not below the state folder"))?;
    let parent = relative.parent().unwrap_or(Path::new(""));
    // The parents are checked first, so the removal below follows no link of a parent.
    super::folders::private_below(state, parent)?;
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    super::folders::private_below(state, relative).map(drop)
}

fn command(launch: &Launch, cgroup: Option<&Path>) -> Command {
    let mut command = match cgroup {
        #[cfg(target_os = "linux")]
        Some(cgroup) => {
            let mut command = Command::new(&launch.launcher);
            command
                .arg(super::tree::linux::LAUNCH_MODE)
                .arg(cgroup.join("cgroup.procs"))
                .arg(std::process::id().to_string())
                .arg(launch.place.join(tree::LAUNCH_FAILURE))
                .arg(&launch.program);
            command
        }
        _ => Command::new(&launch.program),
    };
    command
        .args(&launch.arguments)
        .env_clear()
        .envs(launch.environment.iter().map(|(name, value)| (name, value)))
        .current_dir(&launch.temporary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    command
}

/// The cgroup of the server on Linux; none on macOS.
#[cfg_attr(
    not(target_os = "linux"),
    expect(
        clippy::unnecessary_wraps,
        reason = "Linux can fail; one signature for both"
    )
)]
fn contain(launch: &Launch) -> Result<Option<PathBuf>, Cause> {
    #[cfg(target_os = "linux")]
    {
        super::tree::linux::make(&launch.server.to_string())
            .map(Some)
            .map_err(Cause::ContainmentFailed)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = launch;
        Ok(None)
    }
}

/// Starts the process of `launch` in its tree.
///
/// Call it from a thread that lives as long as the daemon (a worker of the async runtime),
/// never from `spawn_blocking`: on Linux the parent-death signal of the launcher fires when
/// the thread that started it ends ([`super::tree::linux::launch`]).
///
/// # Errors
/// `containment_failed` when the cgroup cannot be made (Linux); `start_failed` for an error of
/// the folders, the ring or the start.
pub fn spawn(launch: &Launch) -> Result<Running, Cause> {
    fresh_temporary(&launch.state, &launch.temporary).map_err(|_| Cause::StartFailed)?;
    match std::fs::remove_file(launch.place.join(tree::LAUNCH_FAILURE)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(Cause::StartFailed),
    }
    let ring = Ring::create(&launch.ring).map_err(|_| Cause::StartFailed)?;
    let cgroup = contain(launch)?;
    let mut child = command(launch, cgroup.as_deref())
        .spawn()
        .map_err(|_| Cause::StartFailed)?;
    let leader = child
        .id()
        .and_then(|id| i32::try_from(id).ok())
        .and_then(tree::identity)
        .ok_or(Cause::StartFailed)?;
    if let Some(output) = child.stdout.take() {
        ring.follow(output);
    }
    if let Some(output) = child.stderr.take() {
        ring.follow(output);
    }
    let running = Running {
        child,
        known: vec![leader.clone()],
        leader,
        cgroup,
        record: launch.place.join(tree::RECORD),
        key: launch.key.clone(),
        ended: false,
    };
    running.record().save(&running.record);
    Ok(running)
}

/// Ends the whole tree at once, also when its main process has already ended.
pub async fn kill(running: &mut Running) {
    let _ = running.poll().await;
    running.end_off_thread().await;
    let _ = running.child.wait().await;
    running.ended = true;
}

/// Asks the tree to end, waits at most 10 seconds for the main process, then ends what is
/// left of the tree.
pub async fn stop(running: &mut Running) {
    let _ = running.poll().await;
    let tree = running.tree();
    let _ = tokio::task::spawn_blocking(move || tree.signal_all(Signal::TERM)).await;
    let _ = tokio::time::timeout(Duration::from_secs(10), running.child.wait()).await;
    kill(running).await;
}
