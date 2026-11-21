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

/// Processes by their ID with the start time that `ps` gave, so that a process that took the ID
/// of one that ended is never taken for it (H9).
pub type Known = BTreeMap<u32, String>;

/// The process table at one time: each process, its parent and its start time.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub rows: Vec<Row>,
    pub starts: Known,
}

impl Table {
    /// The table of `ps -axo pid=,ppid=,lstart=`; a line that does not read is left out.
    #[must_use]
    pub fn read(text: &str) -> Self {
        let mut table = Self::default();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(Ok(pid)), Some(Ok(parent))) = (
                fields.next().map(str::parse::<u32>),
                fields.next().map(str::parse::<u32>),
            ) else {
                continue;
            };
            table.rows.push(Row { pid, parent });
            table
                .starts
                .insert(pid, fields.collect::<Vec<_>>().join(" "));
        }
        table
    }

    /// Each process below `root`, with its start time.
    #[must_use]
    pub fn below(&self, root: u32) -> Known {
        below(&self.rows, root)
            .into_iter()
            .filter_map(|pid| Some((pid, self.starts.get(&pid)?.clone())))
            .collect()
    }

    /// The processes of `known` that still run with the same start time.
    #[must_use]
    pub fn still(&self, known: &Known) -> BTreeSet<u32> {
        known
            .iter()
            .filter(|(pid, start)| self.starts.get(pid) == Some(start))
            .map(|(pid, _)| *pid)
            .collect()
    }
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

/// The processes of `spare` that still run as the same process, with each process below them.
fn spare_with_children(table: &Table, spare: &Known) -> BTreeSet<u32> {
    let mut out = table.still(spare);
    for pid in out.clone() {
        out.extend(below(&table.rows, pid));
    }
    out
}

/// The program `ps` at a fixed path: the holder has no `PATH`.
pub(super) fn ps() -> Option<&'static str> {
    ["/bin/ps", "/usr/bin/ps"]
        .into_iter()
        .find(|path| Path::new(path).exists())
}

/// The process table now; `None` when `ps` is missing or fails, so that a stop never claims an
/// end it could not see.
pub async fn table() -> Option<Table> {
    let output = tokio::process::Command::new(ps()?)
        .args(["-axo", "pid=,ppid=,lstart="])
        .env_clear()
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    let table = Table::read(&String::from_utf8_lossy(&output.stdout));
    (output.status.success() && !table.rows.is_empty()).then_some(table)
}

/// The start time of a process as `ps` says it, so that an old record never ends a process
/// that took an old ID (H9).
pub async fn started(pid: u32) -> Option<String> {
    let output = tokio::process::Command::new(ps()?)
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env_clear()
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

pub(super) fn signal(pid: u32, signal: Signal) {
    if let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) {
        // A process that already ended is what a stop wants: the error is not a fault.
        let _ = kill_process(pid, signal);
    }
}

/// Whether `pid` is a process that still runs. A zombie (it ended; its parent has not reaped it)
/// does not run: on Linux a shell that waits in `read` does not reap a child that was killed.
#[must_use]
pub fn alive(pid: u32) -> bool {
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .is_some_and(|pid| test_kill_process(pid).is_ok())
        && !zombie(pid)
}

/// Whether the system says that `pid` ended and waits to be reaped. Only Linux tells it without
/// a program (`/proc`); elsewhere this says no.
fn zombie(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| zombie_stat(&stat))
}

/// The state of `/proc/<pid>/stat` is the field after the name in parentheses (the name may hold
/// spaces and parentheses, so the last `)` ends it).
fn zombie_stat(stat: &str) -> bool {
    stat.rfind(')')
        .and_then(|end| stat[end + 1..].split_whitespace().next())
        == Some("Z")
}

/// What an end of processes found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ended {
    pub count: u32,
    pub all: bool,
}

/// Ends each process below `root` that is not in `spare` (with the processes below it), and
/// `root` too when `with_root`. The root is stopped first
/// (`SIGSTOP`), so it starts nothing new while the walk is repeated; each process found below it
/// is stopped too, then all get `SIGKILL`. The root then gets `SIGTERM` and `SIGCONT`, so that
/// the program can write its session file, and `SIGKILL` after `grace`. When the root goes on
/// (`with_root` false) it gets `SIGCONT`. A process that the root starts after it goes on, in a
/// group of its own, is not found: that is said in the answer only when the walk cannot run.
pub async fn end(root: u32, with_root: bool, spare: &Known, grace: Duration) -> Ended {
    signal(root, Signal::STOP);
    let mut known: BTreeSet<u32> = BTreeSet::new();
    let mut seen = true;
    for _ in 0..8 {
        let Some(table) = table().await else {
            seen = false;
            break;
        };
        let spared = spare_with_children(&table, spare);
        let new: Vec<u32> = below(&table.rows, root)
            .difference(&known)
            .filter(|pid| !spared.contains(pid))
            .copied()
            .collect();
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
        signal(root, Signal::CONT);
        known.extend(during_grace(root, grace).await);
        signal(root, Signal::KILL);
    } else {
        signal(root, Signal::CONT);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    // The root is reaped by its holder, so a root that ended may still answer a signal test.
    let left = known.iter().filter(|pid| alive(**pid)).count();
    let count = known.len() + usize::from(with_root);
    Ended {
        count: u32::try_from(count).unwrap_or(u32::MAX),
        all: seen && left == 0,
    }
}

/// While the root writes its session file after `SIGTERM`, each process that it starts is found
/// by a walk each half second and ended; the last walk is just before its `SIGKILL`.
async fn during_grace(root: u32, grace: Duration) -> BTreeSet<u32> {
    let deadline = tokio::time::Instant::now() + grace;
    let mut ended = BTreeSet::new();
    loop {
        if let Some(table) = table().await {
            for pid in below(&table.rows, root) {
                signal(pid, Signal::KILL);
                ended.insert(pid);
            }
        }
        if !alive(root) || tokio::time::Instant::now() >= deadline {
            return ended;
        }
        tokio::time::sleep(Duration::from_millis(500).min(grace)).await;
    }
}

/// Ends each process of `known` that still runs as the same process, with the processes below
/// it: what a program left when it ended by itself, which is not below it any more (THR-049).
pub async fn sweep(known: &Known) -> Ended {
    if known.is_empty() {
        return Ended {
            count: 0,
            all: true,
        };
    }
    let Some(table) = table().await else {
        return Ended {
            count: 0,
            all: false,
        };
    };
    let found = spare_with_children(&table, known);
    for pid in &found {
        signal(*pid, Signal::KILL);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let left = found.iter().filter(|pid| alive(**pid)).count();
    Ended {
        count: u32::try_from(found.len()).unwrap_or(u32::MAX),
        all: left == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_049_a_zombie_does_not_run() {
        assert!(zombie_stat("4242 (sh) Z 1 4242 4242 0 -1"));
        assert!(zombie_stat("4242 (a (b) c) Z 1"));
        assert!(!zombie_stat("4242 (sleep) S 1 4242"));
        assert!(!zombie_stat("not a stat"));
    }

    #[test]
    fn thr_049_the_walk_finds_each_process_below_the_root_in_any_group() {
        let table = "  10 1\n  11 10\n  12 11\n  13 12\n  20 1\n  21 20\nbad line\n";
        assert_eq!(below(&rows(table), 10), BTreeSet::from([11, 12, 13]));
        assert_eq!(below(&rows(table), 13), BTreeSet::new());
    }

    #[test]
    fn thr_021_a_spared_server_keeps_its_own_children_and_a_gone_one_spares_nothing() {
        let table = Table::read("10 1 a\n11 10 b\n12 11 c\n13 10 d\n14 13 e\n");
        let spare = Known::from([(11, "b".to_owned()), (99, "z".to_owned())]);
        assert_eq!(
            spare_with_children(&table, &spare),
            BTreeSet::from([11, 12])
        );
    }

    #[test]
    fn h9_a_process_that_took_the_id_of_a_spared_one_is_not_spared() {
        let table = Table::read("10 1 a\n11 10 Fri Nov 20 10:00:01 2026\n");
        let old = Known::from([(11, "Fri Nov 20 09:00:00 2026".to_owned())]);
        assert!(spare_with_children(&table, &old).is_empty());
        assert_eq!(
            table.below(10),
            Known::from([(11, "Fri Nov 20 10:00:01 2026".to_owned())])
        );
    }

    #[test]
    fn thr_049_a_loop_in_the_table_does_not_hang_the_walk() {
        let table = "1 2\n2 1\n3 1\n";
        assert_eq!(below(&rows(table), 1), BTreeSet::from([2, 3]));
    }
}
