//! Ends every process a build step started, including those that left the step's process group
//! with `setsid` (`pg_ctl` does; so can a hostile build script). On Linux this process is the
//! child subreaper of its descendants, so an escaped process is re-parented here rather than to
//! `init`; each step's processes carry a marker variable, and after the step, and on cancel,
//! every live descendant with the step's marker is killed and reaped (BLD-026, S-1).
//!
//! On macOS there is no subreaper: only the step's process group is killed, and a process that
//! called `setsid` outlives the step (a known limit).
//!
//! The parsers are pure; reading `/proc` and sending signals is the edge.

/// The variable that marks a build step's processes; its value is the step's own.
pub const MARKER: &str = "ANIMA_BUILD_STEP";

/// One process as `/proc/<pid>/stat` describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Process {
    pub pid: i32,
    pub parent: i32,
    pub group: i32,
    pub session: i32,
    pub zombie: bool,
}

/// Reads `/proc/<pid>/stat`. The command name is in parentheses and may hold anything, spaces
/// and parentheses included, so the fields are read after the last `)`.
pub fn process_of(pid: i32, stat: &str) -> Option<Process> {
    let (_, fields) = stat.rsplit_once(')')?;
    let mut fields = fields.split_whitespace();
    let state = fields.next()?;
    let parent = fields.next()?.parse().ok()?;
    let group = fields.next()?.parse().ok()?;
    let session = fields.next()?.parse().ok()?;
    Some(Process {
        pid,
        parent,
        group,
        session,
        zombie: state == "Z" || state == "X",
    })
}

/// Every process below `root` in `all`, children before grandchildren.
pub fn descendants(root: i32, all: &[Process]) -> Vec<Process> {
    let mut found: Vec<Process> = Vec::new();
    let mut parents = vec![root];
    while let Some(parent) = parents.pop() {
        for process in all.iter().filter(|process| process.parent == parent) {
            if process.pid != root && !found.iter().any(|seen| seen.pid == process.pid) {
                found.push(*process);
                parents.push(process.pid);
            }
        }
    }
    found
}

/// Whether the `NUL`-separated environment `environ` holds exactly `MARKER=<value>`.
pub fn marked(environ: &[u8], value: &str) -> bool {
    let wanted = format!("{MARKER}={value}");
    environ
        .split(|byte| *byte == 0)
        .any(|entry| entry == wanted.as_bytes())
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{collections::HashSet, fs, io::Read, sync::Once, thread, time::Duration};

    use nix::{
        sys::{
            signal::{Signal, kill},
            wait::{WaitPidFlag, waitpid},
        },
        unistd::Pid,
    };

    use super::{Process, descendants, marked, process_of};

    /// An environment is read up to this many bytes; the marker is near its end at most this far.
    const ENVIRON_MAX_BYTES: u64 = 1 << 20;
    const ROUNDS: usize = 200;
    const PAUSE: Duration = Duration::from_millis(10);

    /// Makes this process the child subreaper of its descendants; once per process.
    pub fn become_subreaper() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = nix::sys::prctl::set_child_subreaper(true);
        });
    }

    fn processes() -> Vec<Process> {
        let Ok(entries) = fs::read_dir("/proc") else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<i32>().ok())
            .filter_map(|pid| {
                process_of(pid, &fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
            })
            .collect()
    }

    fn carries(pid: i32, value: &str) -> bool {
        let mut environ = Vec::new();
        fs::File::open(format!("/proc/{pid}/environ"))
            .and_then(|file| file.take(ENVIRON_MAX_BYTES).read_to_end(&mut environ))
            .is_ok_and(|_| marked(&environ, value))
    }

    fn me() -> i32 {
        i32::try_from(std::process::id()).unwrap_or(i32::MAX)
    }

    /// Sends SIGKILL to every live descendant that carries the marker `value`; returns them.
    pub fn kill_marked(value: &str) -> Vec<i32> {
        let all = processes();
        let live: Vec<i32> = descendants(me(), &all)
            .into_iter()
            .filter(|process| !process.zombie && carries(process.pid, value))
            .map(|process| process.pid)
            .collect();
        for pid in &live {
            let _ = kill(Pid::from_raw(*pid), Signal::SIGKILL);
        }
        live
    }

    /// Kills every descendant with the marker `value` until none is left, and reaps those that
    /// were re-parented here. Only processes seen with the marker, of the step's process
    /// `group`, or of another session (they called `setsid`) are reaped, so a child the app
    /// waits for elsewhere is never taken from it. Run it after the step's own process was waited
    /// for. Blocking: at most `ROUNDS` pauses.
    pub fn sweep(value: &str, group: Option<i32>) {
        let mut killed: HashSet<i32> = HashSet::new();
        for _ in 0..ROUNDS {
            let live = kill_marked(value);
            killed.extend(&live);
            let me = me();
            let all = processes();
            let session = all
                .iter()
                .find(|process| process.pid == me)
                .map(|process| process.session);
            let ours = |process: &Process| {
                killed.contains(&process.pid)
                    || Some(process.group) == group
                    || (process.parent == me && Some(process.session) != session)
            };
            let waiting: Vec<Process> = all.into_iter().filter(|process| ours(process)).collect();
            for process in waiting
                .iter()
                .filter(|process| process.parent == me && process.zombie)
            {
                let _ = waitpid(Pid::from_raw(process.pid), Some(WaitPidFlag::WNOHANG));
            }
            if live.is_empty() && waiting.iter().all(|process| process.parent != me) {
                return;
            }
            thread::sleep(PAUSE);
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::{become_subreaper, kill_marked, sweep};

#[cfg(not(target_os = "linux"))]
pub fn become_subreaper() {}

#[cfg(not(target_os = "linux"))]
pub fn kill_marked(_value: &str) -> Vec<i32> {
    Vec::new()
}

#[cfg(not(target_os = "linux"))]
pub fn sweep(_value: &str, _group: Option<i32>) {}
