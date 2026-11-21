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

/// Ends each process of this user that carries the mark of `thread`. `all` is false when the
/// environments could not be read or a marked process still runs after its `SIGKILL`.
pub async fn end_marked(thread: &str) -> Ended {
    let Some(marked) = carrying(thread).await else {
        return Ended {
            count: 0,
            all: false,
        };
    };
    for pid in &marked {
        tree::signal(*pid, Signal::KILL);
    }
    if !marked.is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let left = marked.iter().filter(|pid| tree::alive(**pid)).count();
    Ended {
        count: u32::try_from(marked.len()).unwrap_or(u32::MAX),
        all: left == 0,
    }
}

/// The processes whose environment holds the mark of `thread`; `None` when it cannot be read.
/// On a Mac `ps -E` shows the environment of the processes of this user; on Linux each
/// `/proc/<pid>/environ` of this user can be read.
#[cfg(target_os = "macos")]
async fn carrying(thread: &str) -> Option<BTreeSet<u32>> {
    let output = tokio::process::Command::new(tree::ps()?)
        .args(["-axww", "-E", "-o", "pid=,command="])
        .env_clear()
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    output
        .status
        .success()
        .then(|| marked_in(&String::from_utf8_lossy(&output.stdout), thread))
}

#[cfg(not(target_os = "macos"))]
async fn carrying(thread: &str) -> Option<BTreeSet<u32>> {
    let wanted = format!("{MARK}={thread}");
    let mut found = BTreeSet::new();
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
            found.insert(pid);
        }
    }
    Some(found)
}

/// Whether the process `pid` carries the mark of any thread. A process whose environment
/// cannot be read (a program of the system on a Mac) does not.
pub async fn carries_a_mark(pid: u32) -> bool {
    let prefix = format!("{MARK}=");
    environment_words(pid)
        .await
        .is_some_and(|words| words.iter().any(|word| word.starts_with(&prefix)))
}

#[cfg(target_os = "macos")]
async fn environment_words(pid: u32) -> Option<Vec<String>> {
    let output = tokio::process::Command::new(tree::ps()?)
        .args(["-ww", "-E", "-o", "command=", "-p", &pid.to_string()])
        .env_clear()
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.split_whitespace().map(str::to_owned).collect())
}

#[cfg(not(target_os = "macos"))]
async fn environment_words(pid: u32) -> Option<Vec<String>> {
    let bytes = tokio::fs::read(format!("/proc/{pid}/environ")).await.ok()?;
    Some(
        bytes
            .split(|byte| *byte == 0)
            .map(|one| String::from_utf8_lossy(one).into_owned())
            .collect(),
    )
}

/// The processes of a `ps -E` table whose line holds the mark of `thread` as a word of its own.
#[must_use]
pub fn marked_in(table: &str, thread: &str) -> BTreeSet<u32> {
    let wanted = format!("{MARK}={thread}");
    table
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let pid = words.next()?.parse::<u32>().ok()?;
            words.any(|word| word == wanted).then_some(pid)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_049_a_process_is_marked_only_by_the_whole_mark_of_its_thread() {
        let table = "  10 /bin/sleep 9 HOME=/h AGENT_THREAD=t-1\n  11 /bin/sh AGENT_THREAD=t-10\n 12 x AGENT_THREAD=t-1x\nnot a line\n";
        assert_eq!(marked_in(table, "t-1"), BTreeSet::from([10]));
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
        let ended = end_marked(&thread).await;
        let _ = reaper.join();
        assert_eq!(
            ended,
            Ended {
                count: 1,
                all: true
            }
        );
        assert_eq!(
            end_marked(&thread).await,
            Ended {
                count: 0,
                all: true
            }
        );
    }
}
