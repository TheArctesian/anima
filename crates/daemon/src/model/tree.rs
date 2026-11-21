//! The process tree of a server (Section 9; R-NEW-containment): the set of processes that the
//! loopback check reads and that a stop ends.
//!
//! On Linux the tree is one cgroup v2 per server below the cgroup of the daemon ([`linux`]):
//! the server joins it before its program runs, so no `setsid` or double fork leaves it. When
//! the cgroup cannot be made, the start is refused (`containment_failed`), with no fallback. On
//! macOS the tree is the walk of the parent processes from the server, and each process seen
//! at a poll is kept by its identity ([`walk`]); this is best effort unless the sandbox is on.
//!
//! Before each signal the daemon checks the identity of the process (its id and its start
//! time), so a process id that the system gave to a different program is never signalled. The
//! identities and the cgroup are written to a record in the state folder at each poll, and
//! [`sweep`] ends what a record names when the daemon starts.

#[cfg(target_os = "linux")]
pub mod linux;
pub mod mounts;
pub mod walk;

use std::io;
use std::path::{Path, PathBuf};

use rustix::process::{Pid, Signal};
use serde::{Deserialize, Serialize};

/// The name of the record of a tree in the folder of its server.
pub const RECORD: &str = "tree.json";
/// The file in the folder of a server where the launcher writes its exit code before it exits
/// at a failed step, so an exit code of the runtime is never read as one of the launcher.
pub const LAUNCH_FAILURE: &str = "launch-failure";

/// One process by its id and its start time, as the system gives it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identity {
    pub pid: i32,
    pub start: String,
}

/// The most bytes of the record of a tree that the daemon reads.
pub const RECORD_BYTES: usize = 64 << 10;
/// The most bytes of the failure file of the launcher that the daemon reads.
pub const LAUNCH_FAILURE_BYTES: usize = 4 << 10;

/// Reads the file `path` that a runtime may have replaced: no link is followed (the folder or
/// the file), a FIFO or a device is refused without a wait (`O_NONBLOCK`, then a regular file
/// only), and a file over `cap` bytes is refused. `None` for each refusal or error.
#[must_use]
pub fn read_capped(path: &Path, cap: usize) -> Option<Vec<u8>> {
    use rustix::fs::{FileType, Mode, OFlags};
    use std::io::Read as _;
    let (folder, name) = (path.parent()?, path.file_name()?);
    let flags = OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::RDONLY;
    let dir = rustix::fs::open(folder, flags | OFlags::DIRECTORY, Mode::empty()).ok()?;
    let fd = rustix::fs::openat(&dir, name, flags | OFlags::NONBLOCK, Mode::empty()).ok()?;
    let stat = rustix::fs::fstat(&fd).ok()?;
    let size = usize::try_from(stat.st_size).ok()?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || size > cap {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::from(fd)
        .take(u64::try_from(cap).ok()? + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= cap).then_some(bytes)
}

/// The start time of process `pid` now, or `None` when no such process exists.
#[must_use]
pub fn start_of(pid: i32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        linux::start_of(pid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        walk::start_of(pid)
    }
}

/// The identity of process `pid` now.
#[must_use]
pub fn identity(pid: i32) -> Option<Identity> {
    start_of(pid).map(|start| Identity { pid, start })
}

/// Sends `signal` to the process of `identity` only when the process with that id has the
/// same start time now. `false` when it does not, or no longer exists.
#[must_use]
pub fn signal(identity: &Identity, signal: Signal) -> bool {
    let Some(pid) = Pid::from_raw(identity.pid) else {
        return false;
    };
    if start_of(identity.pid).as_deref() != Some(identity.start.as_str()) {
        return false;
    }
    rustix::process::kill_process(pid, signal).is_ok()
}

/// What the record of a tree holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub known: Vec<Identity>,
    pub cgroup: Option<PathBuf>,
}

impl Record {
    /// Writes the record to `path` (mode 0600, no link followed). A failure is not fatal: the
    /// record only helps the sweep after a crash.
    pub fn save(&self, path: &Path) {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let Ok(text) = serde_json::to_vec(self) else {
            return;
        };
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed())
            .open(path);
        if let Ok(mut file) = file {
            let _ = file.write_all(&text);
        }
    }

    fn load(path: &Path) -> Option<Self> {
        serde_json::from_slice(&read_capped(path, RECORD_BYTES)?).ok()
    }

    /// Ends each process of the record whose identity still holds, and its cgroup. It waits and
    /// runs `ps` (macOS), so async code calls it on a blocking thread.
    pub fn end_all(&self) {
        // A record can be written by the runtime when the sandbox is off; only a cgroup of a
        // server of this daemon is ended (S5).
        #[cfg(target_os = "linux")]
        if let Some(cgroup) = self
            .cgroup
            .as_deref()
            .filter(|path| linux::is_server_cgroup(path))
        {
            linux::end_cgroup(cgroup);
        }
        for identity in &self.known {
            let _gone_or_sent = signal(identity, Signal::KILL);
        }
    }
}

/// Ends the processes and the cgroups that the records below `servers` name, and removes the
/// records: the daemon starts no server by itself (MW-P5), so each one is an orphan of an
/// earlier run. Gives the number of records swept.
///
/// # Errors
/// An I/O error of the folder; a record that does not read is removed with no signal.
pub fn sweep(servers: &Path) -> io::Result<usize> {
    let entries = match std::fs::read_dir(servers) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let mut swept = 0;
    for entry in entries.filter_map(Result::ok) {
        let record = entry.path().join(RECORD);
        if let Some(found) = Record::load(&record) {
            found.end_all();
            swept += 1;
        }
        let _ = std::fs::remove_file(&record);
    }
    #[cfg(target_os = "linux")]
    linux::sweep_cgroups();
    Ok(swept)
}
