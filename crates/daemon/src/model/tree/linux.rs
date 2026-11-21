//! One cgroup v2 for each server (Linux; R-NEW-containment). The cgroup is below the cgroup of
//! the daemon, which its unit must delegate. The server joins it through the launcher: the
//! daemon program in the fixed mode `__model-launch`, which writes its own id to
//! `cgroup.procs`, sets the parent-death signal as a backstop, and then becomes the runtime
//! with `exec`. No shell runs.

use std::io;
use std::path::{Path, PathBuf};

use super::Identity;

/// The first word of the arguments of the launcher.
pub const LAUNCH_MODE: &str = "__model-launch";
/// The prefix of the name of the cgroup of a server.
pub const PREFIX: &str = "anima-model-";
/// The exit code of the launcher when it cannot join its cgroup or start the runtime.
pub const LAUNCH_FAILED: i32 = 125;
/// The most processes in the cgroup of one server, when the `pids` controller is delegated. A
/// placeholder value of the contract (Section 9), so a fork bomb stops before a stop loses to
/// it on a kernel with no `cgroup.kill`.
pub const PIDS_MAX: u32 = 512;

/// The start time of process `pid` in clock ticks (field 22 of `/proc/<pid>/stat`).
#[must_use]
pub fn start_of(pid: i32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = stat.rsplit_once(')')?.1;
    after.split_whitespace().nth(19).map(str::to_owned)
}

/// The cgroup v2 folder of the daemon.
pub fn own_cgroup() -> Option<PathBuf> {
    let text = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let relative = text.lines().find_map(|line| line.strip_prefix("0::"))?;
    let base = Path::new("/sys/fs/cgroup");
    base.join("cgroup.controllers")
        .exists()
        .then(|| base.join(relative.trim_start_matches('/')))
}

/// Makes a new cgroup for one start of server `name` (a UUID). Each start has its own name
/// (the server and the time of the start), so a cgroup of an earlier start that could not be
/// removed never blocks a new start. Sets `pids.max` when the controller is delegated.
///
/// # Errors
/// An I/O error when the system has no cgroup v2 or does not delegate it to this user.
pub fn make(name: &str) -> io::Result<PathBuf> {
    let parent = own_cgroup().ok_or_else(|| io::Error::other("no cgroup v2 for the daemon"))?;
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = parent.join(format!("{PREFIX}{name}-{start}"));
    std::fs::create_dir(&path)?;
    let limit = path.join("pids.max");
    if limit.exists() {
        std::fs::write(&limit, PIDS_MAX.to_string())?;
    }
    Ok(path)
}

/// Whether `cgroup` is a cgroup of a server of this daemon: a folder named with [`PREFIX`]
/// directly below the cgroup of the daemon. The sweep checks each path that a record names.
#[must_use]
pub fn is_server_cgroup(cgroup: &Path) -> bool {
    let named = cgroup
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(PREFIX));
    named && own_cgroup().is_some_and(|own| cgroup.parent() == Some(own.as_path()))
}

/// The processes of the cgroup now, by their identities.
///
/// # Errors
/// An I/O error when `cgroup.procs` does not read; the caller then treats the server as
/// exposed (fails closed).
pub fn members(cgroup: &Path) -> io::Result<Vec<Identity>> {
    let text = std::fs::read_to_string(cgroup.join("cgroup.procs"))?;
    Ok(text
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter_map(super::identity)
        .collect())
}

/// Ends each process of the cgroup (with `cgroup.kill`, and one signal to each member whose
/// identity holds), then removes the cgroup. Waits up to 1 second, so it runs on a blocking
/// thread. A cgroup that does not empty is logged and left; the next start uses a new name.
pub fn end_cgroup(cgroup: &Path) {
    let _ = std::fs::write(cgroup.join("cgroup.kill"), "1");
    for member in members(cgroup).unwrap_or_default() {
        let _gone_or_sent = super::signal(&member, rustix::process::Signal::KILL);
    }
    for _ in 0..50 {
        if std::fs::remove_dir(cgroup).is_ok() || !cgroup.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    tracing::warn!(cgroup = %cgroup.display(), "a cgroup of a model server did not empty; left");
}

/// Ends and removes each cgroup of a server below the cgroup of the daemon.
pub fn sweep_cgroups() {
    let Some(parent) = own_cgroup() else { return };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if entry.file_name().to_string_lossy().starts_with(PREFIX) {
            end_cgroup(&entry.path());
        }
    }
}

/// The launcher: `__model-launch <cgroup.procs> <daemon pid> <program> <arguments…>`. It never
/// returns. The environment is the one that the daemon gave it, already the fixed list.
///
/// The parent-death signal fires when the *thread* that started the launcher ends, not the
/// daemon process. So the daemon must start servers from a thread that lives as long as the
/// daemon (a worker of the async runtime), never from `spawn_blocking`, whose threads end
/// after a time with no work and would kill each server with them.
pub fn launch(arguments: &[String]) -> ! {
    use std::os::unix::process::CommandExt as _;
    let (Some(procs), Some(daemon), Some(program)) =
        (arguments.first(), arguments.get(1), arguments.get(2))
    else {
        std::process::exit(LAUNCH_FAILED);
    };
    let joined = Path::new(procs)
        .file_name()
        .is_some_and(|name| name == "cgroup.procs")
        && std::fs::write(procs, std::process::id().to_string()).is_ok();
    let backstop =
        rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL));
    // The daemon may have ended before the signal was set: then nothing would end the runtime.
    let parent = rustix::process::getppid().map(rustix::process::Pid::as_raw_nonzero);
    let alive = parent.is_some_and(|pid| daemon.parse().ok() == Some(pid.get()));
    if !joined || backstop.is_err() || !alive {
        std::process::exit(LAUNCH_FAILED);
    }
    let _error = std::process::Command::new(program)
        .args(&arguments[3..])
        .exec();
    std::process::exit(LAUNCH_FAILED);
}
