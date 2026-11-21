//! Whether a client of the runner is a process of an agent: a process below the holder, or a
//! process that carries the mark of a thread (THR-049). Such a client may read and write to a
//! thread, but it may not answer a request, raise the limit or start a thread: an agent with a
//! shell must not approve itself (Medium 3 of the checklist read).

use std::collections::BTreeMap;
use std::path::Path;

/// Whether the process `pid` is of an agent of the holder whose folder is `holder_dir`.
/// When the answer cannot be known, the client is taken for an agent: a peer with no process
/// ID, a process that ended before it was read (a child that connects and exits), or one whose
/// environment cannot be read. Such a client can still read and write; only the person's calls
/// are refused (R-315).
pub async fn of_an_agent(pid: Option<i32>, holder_dir: &Path) -> bool {
    let Some(pid) = pid
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 1)
    else {
        return true;
    };
    if crate::threads::marker::mark_of(pid).await != Some(false) {
        return true;
    }
    let Some(table) = commands().await else {
        return true;
    };
    !table.contains_key(&pid) || below_the_holder(&table, pid, &holder_mark(holder_dir))
}

/// The end of the command of the holder of `holder_dir`, as `ps` shows it.
fn holder_mark(holder_dir: &Path) -> String {
    format!(" holder {}", holder_dir.display())
}

/// Each process with its parent and its command.
type Commands = BTreeMap<u32, (u32, String)>;

async fn commands() -> Option<Commands> {
    let output = crate::threads::tree::ps_output(&["-axww", "-o", "pid=,ppid=,command="]).await?;
    Some(table(&String::from_utf8_lossy(&output)))
}

/// The rows of `ps -axww -o pid=,ppid=,command=`; a line that does not read is left out.
#[must_use]
pub fn table(text: &str) -> Commands {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, rest) = line.split_once(char::is_whitespace)?;
            let rest = rest.trim_start();
            let (parent, command) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            Some((
                pid.parse().ok()?,
                (parent.parse().ok()?, command.trim().to_owned()),
            ))
        })
        .collect()
}

/// Whether an ancestor of `pid` is the holder: its command ends with `holder_mark`.
#[must_use]
pub fn below_the_holder(table: &Commands, pid: u32, holder_mark: &str) -> bool {
    let mut at = pid;
    for _ in 0..64 {
        let Some((parent, _)) = table.get(&at) else {
            return false;
        };
        if *parent <= 1 {
            return false;
        }
        if table
            .get(parent)
            .is_some_and(|(_, command)| command.ends_with(holder_mark))
        {
            return true;
        }
        at = *parent;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_049_a_process_below_the_holder_is_of_an_agent_and_a_terminal_is_not() {
        let rows = table(
            "  1 0 /sbin/launchd\n 50 1 /Applications/Terminal\n 51 50 -zsh\n 52 51 anima agents approve x y\n \
             60 1 /u/anima-daemon run\n 61 60 /u/anima-daemon holder /s/threads\n 62 61 claude -p\n \
             63 62 /bin/bash -c anima agents approve a b\n 64 63 anima agents approve a b\n",
        );
        let mark = holder_mark(Path::new("/s/threads"));
        assert!(below_the_holder(&rows, 64, &mark));
        assert!(below_the_holder(&rows, 62, &mark));
        assert!(!below_the_holder(&rows, 52, &mark));
        assert!(
            !below_the_holder(&rows, 61, &mark),
            "the holder is not below itself"
        );
        assert!(!below_the_holder(&rows, 99, &mark), "an unknown process");
    }

    #[tokio::test]
    async fn thr_049_a_client_that_cannot_be_read_is_taken_for_an_agent() {
        let dir = Path::new("/no/such/threads");
        assert!(of_an_agent(None, dir).await, "no process ID");
        assert!(of_an_agent(Some(0), dir).await, "process ID 0");
        let mut child = std::process::Command::new("/usr/bin/true")
            .spawn()
            .expect("a child");
        let gone = child.id();
        child.wait().expect("it ended");
        assert!(
            of_an_agent(i32::try_from(gone).ok(), dir).await,
            "a process that ended before it was read"
        );
        if std::env::var_os(crate::threads::marker::MARK).is_none() {
            let me = i32::try_from(std::process::id()).ok();
            assert!(!of_an_agent(me, dir).await, "this test is not an agent");
        }
    }
}
