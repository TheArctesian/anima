//! The mark of a thread in the environment of its program (THR-049). Each process that the
//! program starts takes the environment with it, also a process that left the tree on purpose
//! (a daemon that the agent started). A stop finds such a process by its mark and ends it, so
//! that "all ended" is said only when no marked process is found.
//!
//! Known limit: on a Mac, `ps -E` does not show the environment of a program of the system
//! (`/bin/sh`, `/usr/bin/…`), so such a process that left the tree is not found there. On Linux
//! each process of this user is read. An agent can also remove the mark from its environment.

use std::collections::BTreeSet;

use rustix::process::Signal;

use super::tree::{self, Ended};

/// The name of the mark: no name of Anima reaches an agent (THR-040). Its value is the key of
/// the thread, which is no secret.
pub const MARK: &str = "AGENT_THREAD";

/// The environment entry of the mark of `thread`.
#[must_use]
pub fn entry(thread: &str) -> (String, String) {
    (MARK.to_owned(), thread.to_owned())
}

/// What an end of marked processes keeps.
pub struct Keep<'a> {
    /// The program, when only its children end: it carries the mark too.
    pub program: Option<u32>,
    /// The start of the program, when only its children end: a marked process that started
    /// before it is not of this run of the program, and is left (THR-049).
    pub since: Option<&'a str>,
    /// The servers that ran before the turn, with each process below them (THR-021).
    pub spare: &'a tree::Known,
}

/// The marked processes that were found, and whether a marked line could not be read as one.
pub struct Found {
    pub pids: BTreeSet<u32>,
    pub unsure: bool,
}

/// Ends each process of this user that carries the mark of `thread`, except what `keep` names.
/// `all` is false when the environments or the table could not be read, or a marked process
/// still runs after its `SIGKILL`.
pub async fn end_marked(thread: &str, keep: &Keep<'_>) -> Ended {
    let Some(Found {
        pids: mut marked,
        unsure,
    }) = carrying(thread).await
    else {
        return Ended {
            count: 0,
            all: false,
        };
    };
    let mut unproven = unsure;
    if let Some(program) = keep.program {
        marked.remove(&program);
    }
    if !keep.spare.is_empty() || keep.since.is_some() {
        if let Some(table) = tree::table().await {
            // A spared server and each process below it are kept, as the walk of the tree
            // keeps them.
            let kept = tree::spare_with_children(&table, keep.spare);
            let since = keep.since.and_then(tree::start_time);
            if keep.since.is_some() && since.is_none() {
                // The start of the program does not read: no process is proved to be of it.
                unproven = true;
                marked.clear();
            }
            marked.retain(|pid| !kept.contains(pid) && started_since(&table, *pid, since));
        } else {
            // Without the table nothing proves which process is spared or how old it is:
            // nothing that may be kept is ended, and "all ended" is not said.
            unproven = true;
            if keep.since.is_some() {
                marked.clear();
            } else {
                marked.retain(|pid| !keep.spare.contains_key(pid));
            }
        }
    }
    for pid in &marked {
        tree::signal(*pid, Signal::KILL);
    }
    if !marked.is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let left = marked.iter().filter(|pid| tree::alive(**pid)).count();
    Ended {
        count: u32::try_from(marked.len()).unwrap_or(u32::MAX),
        all: left == 0 && !unproven,
    }
}

/// Whether the process `pid` of `table` started at or after `since`; a process whose start
/// cannot be read has ended, and an end of it changes nothing.
fn started_since(table: &tree::Table, pid: u32, since: Option<chrono::NaiveDateTime>) -> bool {
    let Some(since) = since else {
        return true;
    };
    table
        .starts
        .get(&pid)
        .and_then(|start| tree::start_time(start))
        .is_none_or(|start| start >= since)
}

/// The processes whose environment holds the mark of `thread`; `None` when it cannot be read.
/// On a Mac `ps -E` shows the environment of the processes of this user after their command;
/// the command alone comes from a second `ps`, so that an argument never counts as the mark.
/// On Linux each `/proc/<pid>/environ` of this user can be read.
#[cfg(target_os = "macos")]
async fn carrying(thread: &str) -> Option<Found> {
    let commands = tree::ps_output(&["-axww", "-o", "pid=,command="]).await?;
    let with_env = tree::ps_output(&["-axww", "-E", "-o", "pid=,command="]).await?;
    Some(marked_in(
        &String::from_utf8_lossy(&commands),
        &String::from_utf8_lossy(&with_env),
        thread,
    ))
}

#[cfg(not(target_os = "macos"))]
async fn carrying(thread: &str) -> Option<Found> {
    let wanted = format!("{MARK}={thread}");
    let mut pids = BTreeSet::new();
    let mut folders = tokio::fs::read_dir("/proc").await.ok()?;
    while let Ok(Some(folder)) = folders.next_entry().await {
        let Some(pid) = folder
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        // A process of a different user cannot be read and is not of this thread.
        if let Ok(bytes) = tokio::fs::read(folder.path().join("environ")).await
            && bytes
                .split(|byte| *byte == 0)
                .any(|one| one == wanted.as_bytes())
        {
            pids.insert(pid);
        }
    }
    Some(Found {
        pids,
        unsure: false,
    })
}

/// Whether the process `pid` is known to carry the mark of any thread.
pub async fn carries_a_mark(pid: u32) -> bool {
    mark_of(pid).await == Some(true)
}

/// Whether the process `pid` carries a mark of a thread; `None` when its environment cannot be
/// read (the process ended, or the system hides it).
pub async fn mark_of(pid: u32) -> Option<bool> {
    let prefix = format!("{MARK}=");
    let words = environment_words(pid).await?;
    (!words.is_empty()).then(|| words.iter().any(|word| word.starts_with(&prefix)))
}

#[cfg(target_os = "macos")]
async fn environment_words(pid: u32) -> Option<Vec<String>> {
    let output = tree::ps_output(&["-ww", "-E", "-o", "command=", "-p", &pid.to_string()]).await?;
    let text = String::from_utf8_lossy(&output);
    Some(text.split_whitespace().map(str::to_owned).collect())
}

#[cfg(not(target_os = "macos"))]
async fn environment_words(pid: u32) -> Option<Vec<String>> {
    let bytes = tokio::fs::read(format!("/proc/{pid}/environ")).await.ok()?;
    Some(
        bytes
            .split(|byte| *byte == 0)
            .filter(|one| !one.is_empty())
            .map(|one| String::from_utf8_lossy(one).into_owned())
            .collect(),
    )
}

/// The processes whose environment holds the mark of `thread` as a word of its own: each line
/// of `with_env` (`ps -E`) is its line of `commands` followed by the environment. A line whose
/// command changed between the two tables, and that holds the mark, makes the answer unsure.
#[must_use]
pub fn marked_in(commands: &str, with_env: &str, thread: &str) -> Found {
    let wanted = format!("{MARK}={thread}");
    let command_of: std::collections::BTreeMap<u32, &str> = commands
        .lines()
        .filter_map(|line| {
            let (pid, rest) = line.trim_start().split_once(' ')?;
            Some((pid.parse().ok()?, rest.trim()))
        })
        .collect();
    let mut found = Found {
        pids: BTreeSet::new(),
        unsure: false,
    };
    for line in with_env.lines() {
        let Some((pid, rest)) = line.trim_start().split_once(' ') else {
            continue;
        };
        let (Ok(pid), rest) = (pid.parse::<u32>(), rest.trim()) else {
            continue;
        };
        if !rest.split_whitespace().any(|word| word == wanted) {
            continue;
        }
        match command_of
            .get(&pid)
            .and_then(|command| rest.strip_prefix(command))
        {
            Some(environment) if environment.split_whitespace().any(|word| word == wanted) => {
                found.pids.insert(pid);
            }
            Some(_) => {}
            None => found.unsure = true,
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_049_a_process_is_marked_only_by_the_whole_mark_of_its_thread() {
        let commands = "  10 /bin/sleep 9\n  11 /bin/sh\n 12 x\n";
        let with_env = "  10 /bin/sleep 9 HOME=/h AGENT_THREAD=t-1\n  11 /bin/sh AGENT_THREAD=t-10\n 12 x AGENT_THREAD=t-1x\nnot a line\n";
        let found = marked_in(commands, with_env, "t-1");
        assert_eq!(found.pids, BTreeSet::from([10]));
        assert!(!found.unsure);
    }

    #[test]
    fn thr_049_an_argument_that_looks_like_the_mark_is_not_the_mark() {
        let commands = "  20 /usr/bin/grep AGENT_THREAD=t-1\n  21 /bin/sh -c x\n";
        let with_env = "  20 /usr/bin/grep AGENT_THREAD=t-1 HOME=/h\n  21 /bin/sh -c x AGENT_THREAD=t-1\n  22 /new AGENT_THREAD=t-1\n";
        let found = marked_in(commands, with_env, "t-1");
        assert_eq!(
            found.pids,
            BTreeSet::from([21]),
            "only the environment counts"
        );
        assert!(
            found.unsure,
            "a marked process missing from the first table is not proved"
        );
    }

    #[test]
    fn thr_049_after_a_turn_only_a_process_started_with_or_after_the_program_is_ended() {
        let table = tree::Table::read(
            "  30 1 Sun Nov  1 09:05:02 2026\n  31 1 Sun Nov  1 09:05:03 2026\n  32 1 Sun Nov  1 09:06:00 2026\n",
        );
        let since = tree::start_time("Sun Nov  1 09:05:03 2026");
        assert!(since.is_some(), "an lstart with a day of one digit reads");
        assert!(!started_since(&table, 30, since), "older than the program");
        assert!(started_since(&table, 31, since), "the same second");
        assert!(started_since(&table, 32, since));
        assert!(
            started_since(&table, 33, since),
            "gone: an end changes nothing"
        );
        assert!(
            started_since(&table, 30, None),
            "a stop ends each marked process"
        );
    }

    fn none() -> Keep<'static> {
        static EMPTY: tree::Known = tree::Known::new();
        Keep {
            program: None,
            since: None,
            spare: &EMPTY,
        }
    }

    /// A process to end: the test program itself, waiting (a program of the system would not
    /// show its environment on a Mac).
    #[test]
    #[ignore = "started by the test of the mark, which ends it"]
    fn waits_to_be_ended() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    #[tokio::test]
    async fn thr_049_a_process_that_left_the_tree_is_found_by_its_mark_and_ended() {
        let thread = format!("test-{}", std::process::id());
        let child = std::process::Command::new(std::env::current_exe().expect("the test program"))
            .args([
                "--ignored",
                "--exact",
                "threads::marker::tests::waits_to_be_ended",
            ])
            .env(MARK, &thread)
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("the test program starts");
        let pid = child.id();
        // Reaped at once when it ends, as the system reaps a process that left its tree.
        let reaper = std::thread::spawn(move || {
            let mut child = child;
            child.wait()
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(
            carries_a_mark(pid).await,
            "a client of the runner with the mark is of an agent"
        );
        assert!(!carries_a_mark(std::process::id()).await);
        let ended = end_marked(&thread, &none()).await;
        let _ = reaper.join();
        assert_eq!(
            ended,
            Ended {
                count: 1,
                all: true
            }
        );
        assert_eq!(
            end_marked(&thread, &none()).await,
            Ended {
                count: 0,
                all: true
            }
        );
    }
}
