//! A program that does not answer (TH97): the runner asks each copy for its version before a
//! start, and gives a started program a bounded time for its first line of protocol. In both
//! cases the person gets words that name the program, its path and what to try, never a thread
//! that says "starting" for ever.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use agent_threads::control::RunState;
use agent_threads::drive::Program;
use uuid::Uuid;

/// The time a program has to answer `--version`.
const PROBE_WAIT: Duration = Duration::from_secs(20);
/// The time a different copy has to answer, when the chosen one did not.
const OTHER_WAIT: Duration = Duration::from_secs(5);
/// How long a copy that did not answer is not asked again: a second start in this time gets
/// the same words at once.
const SILENT_KEPT: Duration = Duration::from_secs(60);
/// The time a started program has for its first line of protocol. Codex loads its plugins
/// first (about 90 s on a busy Mac, 2026-11-20); Claude Code reads the Keychain first.
pub const FIRST_ANSWER: Duration = Duration::from_secs(300);

/// The file of a copy, so that a copy that changed is asked again.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    inode: u64,
    modified: Option<std::time::SystemTime>,
}

fn stamp(path: &Path) -> Option<Stamp> {
    use std::os::unix::fs::MetadataExt as _;
    let meta = std::fs::metadata(path).ok()?;
    Some(Stamp {
        inode: meta.ino(),
        modified: meta.modified().ok(),
    })
}

/// What a copy said to `--version`: its first line, or nothing in time.
#[derive(Clone, Debug)]
struct Asked {
    stamp: Option<Stamp>,
    at: Instant,
    version: Option<String>,
}

/// The copies that were asked, and the started threads that wait for their first line.
#[derive(Debug, Default)]
pub struct Silence {
    asked: BTreeMap<PathBuf, Asked>,
    waiting: BTreeMap<Uuid, (Program, PathBuf)>,
    ending: BTreeMap<Uuid, String>,
}

impl Silence {
    /// Asks the copy at `path` for its version (or takes a recent answer). When it does not
    /// answer: the `others` copies that answer, with their versions, for [`not_answered`].
    pub async fn probe(
        &mut self,
        path: &Path,
        others: &[PathBuf],
        env: &[(String, String)],
    ) -> Result<(), Vec<(PathBuf, String)>> {
        if self.version(path, PROBE_WAIT, env).await.is_some() {
            return Ok(());
        }
        let mut answered = Vec::new();
        for other in others.iter().filter(|other| other.as_path() != path) {
            if let Some(version) = self.version(other, OTHER_WAIT, env).await {
                answered.push((other.clone(), version));
            }
        }
        Err(answered)
    }

    /// The first line that `path --version` printed, from the kept answers when the file did
    /// not change; `None` when it did not answer in `wait`.
    async fn version(
        &mut self,
        path: &Path,
        wait: Duration,
        env: &[(String, String)],
    ) -> Option<String> {
        let now = stamp(path);
        if let Some(kept) = self.asked.get(path).filter(|kept| kept.stamp == now)
            && (kept.version.is_some() || kept.at.elapsed() < SILENT_KEPT)
        {
            return kept.version.clone();
        }
        let version = ask_version(path, wait, env).await;
        let at = Instant::now();
        let asked = Asked {
            stamp: now,
            at,
            version: version.clone(),
        };
        self.asked.insert(path.to_owned(), asked);
        version
    }

    /// A started thread: it waits for its first line of protocol.
    pub fn started(&mut self, thread: Uuid, program: Program, path: PathBuf) {
        self.waiting.insert(thread, (program, path));
    }

    /// The threads that are still starting after [`FIRST_ANSWER`], with the words of their end;
    /// a thread that left the state "starting" stops waiting. `now` and the start times come
    /// from the caller.
    pub fn overdue(
        &mut self,
        threads: &[(Uuid, RunState, chrono::DateTime<chrono::Utc>)],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Vec<Uuid> {
        let limit = chrono::Duration::from_std(FIRST_ANSWER).unwrap_or(chrono::Duration::MAX);
        let mut due = Vec::new();
        self.waiting
            .retain(|id, _| threads.iter().any(|(thread, ..)| thread == id));
        for (id, state, started) in threads {
            if *state != RunState::Starting {
                self.waiting.remove(id);
                continue;
            }
            if now - *started < limit {
                continue;
            }
            if let Some((program, path)) = self.waiting.remove(id) {
                self.ending.insert(*id, said_nothing(program, &path));
                due.push(*id);
            }
        }
        due
    }

    /// The words of the end of a thread that the runner ended because it said nothing.
    pub fn ended(&mut self, thread: Uuid) -> Option<String> {
        self.waiting.remove(&thread);
        self.ending.remove(&thread)
    }
}

/// The name of a program for the person.
pub const fn name(program: Program) -> &'static str {
    match program {
        Program::ClaudeCode => "Claude Code",
        Program::Codex => "Codex",
    }
}

/// The words when the chosen copy did not answer `--version`, with the copies that did.
#[must_use]
pub fn not_answered(program: Program, path: &Path, answered: &[(PathBuf, String)]) -> String {
    let mut words = format!(
        "The {} at {} did not answer in {} seconds. Run `{} --version` once in a terminal to see what it waits for, or update it.",
        name(program),
        path.display(),
        PROBE_WAIT.as_secs(),
        path.display(),
    );
    for (other, version) in answered {
        let _ = write!(
            words,
            " A different copy answered: {} ({version}). To use it, start the thread with that path (`anima agents start --program-path {}`).",
            other.display(),
            other.display(),
        );
    }
    words
}

/// The words when a started program said nothing in [`FIRST_ANSWER`].
#[must_use]
pub fn said_nothing(program: Program, path: &Path) -> String {
    format!(
        "The {} at {} said nothing in {} minutes after its start, so the runner ended it. Run `{} --version` once in a terminal to see what it waits for, or update it.",
        name(program),
        path.display(),
        FIRST_ANSWER.as_secs() / 60,
        path.display(),
    )
}

/// The words of the copy that a start chose, when more than one copy was found (TH97).
#[must_use]
pub fn chosen(program: Program, path: &Path, named: bool, copies: &[PathBuf]) -> Option<String> {
    let others: Vec<String> = copies
        .iter()
        .filter(|copy| copy.as_path() != path)
        .map(|copy| copy.display().to_string())
        .collect();
    if others.is_empty() {
        return None;
    }
    let why = if named {
        "the copy that the start named"
    } else {
        "the first on the PATH of your login, as in a terminal"
    };
    Some(format!(
        "{} runs from {} ({why}). Other copies on this computer: {}.",
        name(program),
        path.display(),
        others.join(", ")
    ))
}

/// The first line of `path --version`, in a process group of its own with no input; the group
/// ends at `wait`.
async fn ask_version(path: &Path, wait: Duration, env: &[(String, String)]) -> Option<String> {
    let mut child = tokio::process::Command::new(path)
        .arg("--version")
        .env_clear()
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .ok()?;
    let group = child
        .id()
        .and_then(|pid| i32::try_from(pid).ok())
        .and_then(rustix::process::Pid::from_raw);
    let mut stdout = child.stdout.take()?;
    let read = async {
        use tokio::io::AsyncReadExt as _;
        let mut output = Vec::new();
        let _ = (&mut stdout).take(4096).read_to_end(&mut output).await;
        let status = child.wait().await.ok()?;
        status.success().then_some(output)
    };
    let answer = tokio::time::timeout(wait, read).await;
    if let Some(group) = group {
        // A group that already ended is what this wants: the error is not a fault.
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
    }
    let output = answer.ok()??;
    let text = String::from_utf8_lossy(&output);
    let line: String = text
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|char| !char.is_control())
        .take(80)
        .collect();
    Some(line.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("a script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        path
    }

    #[tokio::test]
    async fn th97_a_copy_that_does_not_answer_is_named_with_a_copy_that_does() {
        let dir = tempfile::tempdir().expect("a folder");
        let silent = script(dir.path(), "codex", "exec sleep 600");
        let other = dir.path().join("app");
        std::fs::create_dir(&other).expect("a folder");
        let answers = script(&other, "codex", "echo 'codex-cli 0.169.0'");
        let env = vec![("PATH".to_owned(), "/usr/bin:/bin".to_owned())];
        let mut silence = Silence::default();
        let started = Instant::now();
        let answered = silence
            .probe(&silent, &[silent.clone(), answers.clone()], &env)
            .await
            .expect_err("the first copy says nothing");
        let words = not_answered(Program::Codex, &silent, &answered);
        assert!(started.elapsed() < PROBE_WAIT + Duration::from_secs(10));
        assert!(words.contains(&format!("The Codex at {}", silent.display())));
        assert!(words.contains(&format!("{} (codex-cli 0.169.0)", answers.display())));
        let again = Instant::now();
        assert!(
            silence.probe(&silent, &[], &env).await.is_err(),
            "the same words"
        );
        assert!(
            again.elapsed() < Duration::from_secs(2),
            "kept, not asked again"
        );
        assert!(silence.probe(&answers, &[], &env).await.is_ok());
    }

    #[test]
    fn th97_a_thread_still_starting_after_the_limit_is_ended_with_words() {
        let mut silence = Silence::default();
        let (late, early, working) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let path = PathBuf::from("/opt/homebrew/bin/codex");
        for id in [late, early, working] {
            silence.started(id, Program::Codex, path.clone());
        }
        let now = chrono::Utc::now();
        let old = now - chrono::Duration::minutes(6);
        let threads = [
            (late, RunState::Starting, old),
            (
                early,
                RunState::Starting,
                now - chrono::Duration::minutes(1),
            ),
            (working, RunState::Working, old),
        ];
        assert_eq!(silence.overdue(&threads, now), [late]);
        assert_eq!(silence.overdue(&threads, now), Vec::<Uuid>::new(), "once");
        let words = silence.ended(late).expect("words");
        assert!(
            words.starts_with("The Codex at /opt/homebrew/bin/codex said nothing in 5 minutes")
        );
        assert_eq!(silence.ended(working), None);
    }

    #[test]
    fn th97_the_chosen_copy_is_named_only_when_there_are_more() {
        let first = PathBuf::from("/opt/homebrew/bin/codex");
        let app = PathBuf::from("/Applications/ChatGPT.app/codex");
        assert_eq!(
            chosen(Program::Codex, &first, false, std::slice::from_ref(&first)),
            None
        );
        let words = chosen(Program::Codex, &first, false, &[first.clone(), app]).expect("words");
        assert!(words.contains("first on the PATH of your login"));
        assert!(words.ends_with("/Applications/ChatGPT.app/codex."));
    }
}
