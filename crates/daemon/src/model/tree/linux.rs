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

/// The start time of process `pid` in clock ticks (field 22 of `/proc/<pid>/stat`).
#[must_use]
pub fn start_of(pid: i32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = stat.rsplit_once(')')?.1;
    after.split_whitespace().nth(19).map(str::to_owned)
}

/// The cgroup v2 folder of the daemon.
fn own_cgroup() -> Option<PathBuf> {
    let text = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let relative = text.lines().find_map(|line| line.strip_prefix("0::"))?;
    let base = Path::new("/sys/fs/cgroup");
    base.join("cgroup.controllers")
        .exists()
        .then(|| base.join(relative.trim_start_matches('/')))
}

/// Makes the cgroup of server `name` (a UUID).
///
/// # Errors
/// An I/O error when the system has no cgroup v2 or does not delegate it to this user.
pub fn make(name: &str) -> io::Result<PathBuf> {
    let parent = own_cgroup().ok_or_else(|| io::Error::other("no cgroup v2 for the daemon"))?;
    let path = parent.join(format!("{PREFIX}{name}"));
    match std::fs::create_dir(&path) {
        Ok(()) => Ok(path),
        // A cgroup of an earlier start of this server: its processes end first.
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            end_cgroup(&path);
            std::fs::create_dir(&path).map(|()| path)
        }
        Err(error) => Err(error),
    }
}

/// The processes of the cgroup now, by their identities.
#[must_use]
pub fn members(cgroup: &Path) -> Vec<Identity> {
    let text = std::fs::read_to_string(cgroup.join("cgroup.procs")).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter_map(super::identity)
        .collect()
}

/// Ends each process of the cgroup (with `cgroup.kill`, and one signal to each member whose
/// identity holds), then removes the cgroup.
pub fn end_cgroup(cgroup: &Path) {
    let _ = std::fs::write(cgroup.join("cgroup.kill"), "1");
    for member in members(cgroup) {
        super::signal(&member, rustix::process::Signal::KILL);
    }
    for _ in 0..50 {
        if std::fs::remove_dir(cgroup).is_ok() || !cgroup.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
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

/// The launcher: `__model-launch <cgroup.procs> <program> <arguments…>`. It never returns. The
/// environment is the one that the daemon gave it, already the fixed list.
pub fn launch(arguments: &[String]) -> ! {
    use std::os::unix::process::CommandExt as _;
    let (Some(procs), Some(program)) = (arguments.first(), arguments.get(1)) else {
        std::process::exit(LAUNCH_FAILED);
    };
    let joined = Path::new(procs)
        .file_name()
        .is_some_and(|name| name == "cgroup.procs")
        && std::fs::write(procs, std::process::id().to_string()).is_ok();
    let backstop =
        rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL));
    if !joined || backstop.is_err() {
        std::process::exit(LAUNCH_FAILED);
    }
    let _error = std::process::Command::new(program)
        .args(&arguments[2..])
        .exec();
    std::process::exit(LAUNCH_FAILED);
}
