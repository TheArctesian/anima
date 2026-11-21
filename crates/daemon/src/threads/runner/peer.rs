//! Whether a client of the runner is a process of an agent: a process below the holder, or a
//! process that carries the mark of a thread (THR-049). Such a client may read and write to a
//! thread, but it may not answer a request, raise the limit or start a thread: an agent with a
//! shell must not approve itself (Medium 3 of the checklist read).

use std::collections::BTreeMap;
use std::path::Path;

/// Whether the process `pid` is of an agent of the holder whose folder is `holder_dir`.
/// A peer whose process is not known (`None`) is not taken for an agent.
pub async fn of_an_agent(pid: Option<i32>, holder_dir: &Path) -> bool {
    let Some(pid) = pid.and_then(|pid| u32::try_from(pid).ok()) else {
        return false;
    };
    if crate::threads::marker::carries_a_mark(pid).await {
        return true;
    }
    let Some(table) = commands().await else {
        return false;
    };
    below_the_holder(&table, pid, &holder_mark(holder_dir))
}

/// The end of the command of the holder of `holder_dir`, as `ps` shows it.
fn holder_mark(holder_dir: &Path) -> String {
    format!(" holder {}", holder_dir.display())
}

/// Each process with its parent and its command.
type Commands = BTreeMap<u32, (u32, String)>;

async fn commands() -> Option<Commands> {
    let ps = ["/bin/ps", "/usr/bin/ps"]
        .into_iter()
        .find(|path| Path::new(path).exists())?;
    let output = tokio::process::Command::new(ps)
        .args(["-axww", "-o", "pid=,ppid=,command="])
        .env_clear()
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;
    output
        .status
        .success()
        .then(|| table(&String::from_utf8_lossy(&output.stdout)))
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
}
