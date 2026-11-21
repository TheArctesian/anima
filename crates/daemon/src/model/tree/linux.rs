//! One cgroup v2 for each server (Linux; R-NEW-containment). The cgroup is below the cgroup of
//! the daemon, which its unit must delegate. The server joins it through the launcher: the
//! daemon program in the fixed mode `__model-launch`, which writes its own id to
//! `cgroup.procs`, sets the parent-death signal as a backstop, and then becomes the runtime
//! with `exec`. No shell runs.

use std::io;
use std::path::{Path, PathBuf};

use daemon_core::model_work::cause::Containment;

use super::Identity;

/// The first word of the arguments of the launcher.
pub const LAUNCH_MODE: &str = "__model-launch";
/// The prefix of the name of the cgroup of a server.
pub const PREFIX: &str = "anima-model-";
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
/// removed never blocks a new start. Sets `cgroup.max.descendants` to 0, so the runtime can
/// make no cgroup below it, and `pids.max` when the controller is delegated. With `nsdelegate`
/// these files of the root of the namespace of the runtime are not writable from inside it.
///
/// # Errors
/// `no_nsdelegate` when the cgroup v2 mount lacks `nsdelegate`; `no_delegation` when the
/// system has no cgroup v2 or does not delegate it to this user.
pub fn make(name: &str) -> Result<PathBuf, Containment> {
    let mountinfo =
        std::fs::read_to_string("/proc/self/mountinfo").map_err(|_| Containment::NoDelegation)?;
    if !super::mounts::cgroup_has_nsdelegate(&mountinfo) {
        return Err(Containment::NoNsdelegate);
    }
    let parent = own_cgroup().ok_or(Containment::NoDelegation)?;
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = parent.join(format!("{PREFIX}{name}-{start}"));
    std::fs::create_dir(&path).map_err(|_| Containment::NoDelegation)?;
    let limits = std::fs::write(path.join("cgroup.max.descendants"), "0").and_then(|()| {
        let pids = path.join("pids.max");
        if pids.exists() {
            std::fs::write(&pids, PIDS_MAX.to_string())
        } else {
            Ok(())
        }
    });
    if limits.is_err() {
        let _ = std::fs::remove_dir(&path);
        return Err(Containment::NoDelegation);
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

/// The processes of the cgroup and of each cgroup below it now, by their identities.
///
/// # Errors
/// An I/O error when a `cgroup.procs` or a folder does not read; the caller then treats the
/// server as exposed (fails closed).
pub fn members(cgroup: &Path) -> io::Result<Vec<Identity>> {
    let mut found = Vec::new();
    for folder in subtree(cgroup)? {
        let text = std::fs::read_to_string(folder.join("cgroup.procs"))?;
        found.extend(
            text.lines()
                .filter_map(|line| line.trim().parse().ok())
                .filter_map(super::identity),
        );
    }
    Ok(found)
}

/// `cgroup` and each cgroup below it, parents before children; no link is followed.
fn subtree(cgroup: &Path) -> io::Result<Vec<PathBuf>> {
    let mut folders = vec![cgroup.to_path_buf()];
    let mut next = 0;
    while let Some(folder) = folders.get(next).cloned() {
        next += 1;
        for entry in std::fs::read_dir(&folder)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                folders.push(entry.path());
            }
        }
    }
    Ok(folders)
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
        // Children first: a cgroup that holds a cgroup cannot be removed.
        for folder in subtree(cgroup).unwrap_or_default().iter().rev() {
            let _ = std::fs::remove_dir(folder);
        }
        if !cgroup.exists() {
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

/// Moves the launcher into a new cgroup namespace whose root is the cgroup of its server
/// (R-NEW-cgroupns): the runtime then sees `/` as its cgroup, and the kernel refuses a move of
/// a process to a cgroup outside that root, such as the cgroup of the daemon. A user of the
/// system may make a cgroup namespace only inside a user namespace of its own, so both are made
/// in one call; the user namespace maps the user and the group of the daemon to themselves.
/// Called with one thread only (a new user namespace needs it).
///
/// # Errors
/// An I/O error when the system does not permit the namespaces; the launcher then refuses the
/// start (`containment_failed`), with no fallback.
fn hide_cgroups() -> io::Result<()> {
    use rustix::thread::UnshareFlags;
    let uid = rustix::process::getuid().as_raw();
    let gid = rustix::process::getgid().as_raw();
    #[expect(
        deprecated,
        reason = "unshare_unsafe needs `unsafe`, which the workspace forbids; its hazard is                   CLONE_FILES and CLONE_FS, which this call does not pass"
    )]
    rustix::thread::unshare(UnshareFlags::NEWUSER | UnshareFlags::NEWCGROUP)?;
    std::fs::write("/proc/self/setgroups", "deny")?;
    std::fs::write("/proc/self/uid_map", format!("{uid} {uid} 1"))?;
    std::fs::write("/proc/self/gid_map", format!("{gid} {gid} 1"))?;
    Ok(())
}

/// Writes the exit code of the failed launcher step `step` to the file `failure`, then exits
/// with it.
fn fail(failure: &str, step: Containment) -> ! {
    let code = step.exit_code().unwrap_or(1);
    let _ = std::fs::write(failure, code.to_string());
    std::process::exit(code);
}

/// The launcher: `__model-launch <cgroup.procs> <daemon pid> <failure file> <program>
/// <arguments…>`. It never returns. At a failed step it writes the exit code of that step
/// ([`Containment::exit_code`]) to the failure file, then exits with it. The environment is the one that the daemon gave it, already the fixed list.
///
/// The parent-death signal fires when the *thread* that started the launcher ends, not the
/// daemon process. So the daemon must start servers from a thread that lives as long as the
/// daemon (a worker of the async runtime), never from `spawn_blocking`, whose threads end
/// after a time with no work and would kill each server with them.
pub fn launch(arguments: &[String]) -> ! {
    use std::os::unix::process::CommandExt as _;
    let (Some(procs), Some(daemon), Some(failure), Some(program)) = (
        arguments.first(),
        arguments.get(1),
        arguments.get(2),
        arguments.get(3),
    ) else {
        std::process::exit(Containment::ExecFailed.exit_code().unwrap_or(1));
    };
    let joined = Path::new(procs)
        .file_name()
        .is_some_and(|name| name == "cgroup.procs")
        && std::fs::write(procs, std::process::id().to_string()).is_ok();
    if !joined {
        fail(failure, Containment::CgroupJoinFailed);
    }
    if rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))
        .is_err()
    {
        fail(failure, Containment::BackstopFailed);
    }
    // The daemon may have ended before the signal was set: then nothing would end the runtime.
    let parent = rustix::process::getppid().map(rustix::process::Pid::as_raw_nonzero);
    if !parent.is_some_and(|pid| daemon.parse().ok() == Some(pid.get())) {
        fail(failure, Containment::ParentGone);
    }
    if hide_cgroups().is_err() {
        fail(failure, Containment::UsernsDenied);
    }
    let _error = std::process::Command::new(program)
        .args(&arguments[4..])
        .exec();
    fail(failure, Containment::ExecFailed);
}
