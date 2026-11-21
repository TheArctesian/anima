//! The processes below the program of a thread, and their end (THR-049). On a Mac and on Linux
//! the holder walks the process tree from the table of `ps`: a command that put itself in a
//! different process group is still below the program (TH10: a command of Claude Code runs in
//! its own group; TH18: the helpers of Codex too). A process that left the tree on purpose (a
//! daemon that the agent started) is not found; the stop then says it could not prove the end.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process, test_kill_process};

/// One row of the process table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub pid: u32,
    pub parent: u32,
}

/// The rows of `ps -axo pid=,ppid=`; a line that does not read is left out.
#[must_use]
pub fn rows(table: &str) -> Vec<Row> {
    table
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace().map(str::parse::<u32>);
            Some(Row {
                pid: fields.next()?.ok()?,
                parent: fields.next()?.ok()?,
            })
        })
        .collect()
}

/// Each process below `root`, at any depth, not `root` itself.
#[must_use]
pub fn below(rows: &[Row], root: u32) -> BTreeSet<u32> {
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for row in rows {
        children.entry(row.parent).or_default().push(row.pid);
    }
    let mut found = BTreeSet::new();
    let mut waiting = vec![root];
    while let Some(pid) = waiting.pop() {
        for child in children.get(&pid).map_or(&[][..], Vec::as_slice) {
            if *child != root && found.insert(*child) {
                waiting.push(*child);
            }
        }
    }
    found
}

/// The program `ps` at a fixed path: the holder has no `PATH`.
fn ps() -> Option<&'static str> {
    ["/bin/ps", "/usr/bin/ps"]
        .into_iter()
        .find(|path| Path::new(path).exists())
}

/// The process table now, as rows.
async fn table() -> Vec<Row> {
    let Some(ps) = ps() else { return Vec::new() };
    let output = tokio::process::Command::new(ps)
        .args(["-axo", "pid=,ppid="])
        .env_clear()
        .kill_on_drop(true)
        .output()
        .await;
    output.map_or_else(
        |_| Vec::new(),
        |out| rows(&String::from_utf8_lossy(&out.stdout)),
    )
}

/// The start time of a process as `ps` says it, so that an old record never ends a process
/// that took an old ID (H9).
pub async fn started(pid: u32) -> Option<String> {
    let output = tokio::process::Command::new(ps()?)
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env_clear()
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

fn signal(pid: u32, signal: Signal) {
    if let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) {
        // A process that already ended is what a stop wants: the error is not a fault.
        let _ = kill_process(pid, signal);
    }
}

#[must_use]
pub fn alive(pid: u32) -> bool {
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .is_some_and(|pid| test_kill_process(pid).is_ok())
}

/// What an end of processes found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ended {
    pub count: u32,
    pub all: bool,
}

/// Ends each process below `root`, and `root` too when `with_root`. First the processes below
/// are stopped (`SIGSTOP`) while the walk is repeated, so that none can start a new one; then
/// they get `SIGKILL`. The root gets `SIGTERM`, so that the program can write its session file,
/// and `SIGKILL` after `grace`.
pub async fn end(root: u32, with_root: bool, grace: Duration) -> Ended {
    let mut known: BTreeSet<u32> = BTreeSet::new();
    for _ in 0..8 {
        let found = below(&table().await, root);
        let new: Vec<u32> = found.difference(&known).copied().collect();
        if new.is_empty() {
            break;
        }
        for pid in &new {
            signal(*pid, Signal::STOP);
        }
        known.extend(new);
    }
    for pid in &known {
        signal(*pid, Signal::KILL);
    }
    if with_root {
        signal(root, Signal::TERM);
        let deadline = tokio::time::Instant::now() + grace;
        while alive(root) && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        signal(root, Signal::KILL);
        known.insert(root);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let left = known
        .iter()
        .filter(|pid| alive(**pid) && !zombie_of(**pid, root))
        .count();
    Ended {
        count: u32::try_from(known.len()).unwrap_or(u32::MAX),
        all: left == 0,
    }
}

/// A root that ended but was not reaped yet still answers a signal test; its holder reaps it.
fn zombie_of(pid: u32, root: u32) -> bool {
    pid == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_049_the_walk_finds_each_process_below_the_root_in_any_group() {
        let table = "  10 1\n  11 10\n  12 11\n  13 12\n  20 1\n  21 20\nbad line\n";
        assert_eq!(below(&rows(table), 10), BTreeSet::from([11, 12, 13]));
        assert_eq!(below(&rows(table), 13), BTreeSet::new());
    }

    #[test]
    fn thr_049_a_loop_in_the_table_does_not_hang_the_walk() {
        let table = "1 2\n2 1\n3 1\n";
        assert_eq!(below(&rows(table), 1), BTreeSet::from([2, 3]));
    }
}
