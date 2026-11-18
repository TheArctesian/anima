//! Reads the session files of Claude Code and Codex of the user the daemon runs as, and gives
//! the session days whose files changed (AIS-018, AIS-022 to AIS-024, K8 Section 4). This is
//! the I/O edge; the lines are read by `daemon_core::agent_usage`.
//!
//! Each file is read from where the last read stopped, and not at all while its size and
//! modification time stay the same (AIS-001). A file is decided on as soon as it names its
//! working folder: a session of an excluded folder or repository is read no further, and none
//! of its counts is kept (AIS-008, AIS-023). Nothing read is logged.

mod files;
mod git;
mod lines;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use daemon_core::agent_usage::{Session, SessionDay, Source, Tally, root_thread};

use crate::config::agent_usage::UsageSettings;
use crate::config::home::HOME_VARIABLE;
use crate::units::plan::SYSTEM_HOME;
use files::{Found, Stamp};
use git::{Bounds, Repository};

/// The most working folders whose repository is remembered between reads.
const MAX_REMEMBERED: usize = 4096;

/// The home folder whose sessions this daemon reads: `HOME`, unless the daemon runs as the
/// systemd system unit, which sets `ANIMA_DAEMON_HOME` and has its state folder as `HOME`, and
/// cannot see a user's files (AIS-024). `None` then, and when `HOME` is not an absolute path.
#[must_use]
pub fn user_home(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if env(HOME_VARIABLE).as_deref() == Some(SYSTEM_HOME) {
        return None;
    }
    let home = PathBuf::from(env("HOME")?);
    (home.is_absolute() && home != Path::new(SYSTEM_HOME)).then_some(home)
}

/// The session files followed so far, and the repositories of their folders.
pub struct AgentSessions {
    home: PathBuf,
    macos: bool,
    settings: UsageSettings,
    followed: HashMap<PathBuf, Followed>,
    repositories: HashMap<String, Repository>,
}

/// One file: where the last read stopped and what it counted.
struct Followed {
    found: Found,
    offset: u64,
    tally: Tally,
    verdict: Verdict,
}

/// Whether a file's session may be counted, known once the file names its working folder.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Unknown,
    Counted,
    Excluded,
}

impl AgentSessions {
    #[must_use]
    pub fn new(home: PathBuf, macos: bool, settings: UsageSettings) -> Self {
        Self {
            home,
            macos,
            settings,
            followed: HashMap::new(),
            repositories: HashMap::new(),
        }
    }

    /// Reads what changed since the last scan and gives every session day of each session one
    /// of whose files changed. Blocking; it ends early when `stopping` is set.
    pub fn scan(&mut self, stopping: &AtomicBool) -> Vec<SessionDay> {
        let found = files::find(&self.home);
        let present: HashSet<&Path> = found.iter().map(|file| file.path.as_path()).collect();
        self.followed
            .retain(|path, _| present.contains(path.as_path()));
        let mut changed = HashSet::new();
        for file in found {
            if stopping.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let path = file.path.clone();
            if self.read(file, stopping) {
                changed.insert(path);
            }
        }
        self.session_days(&changed)
    }

    /// Reads the new lines of `file`; whether it gave anything new to count.
    fn read(&mut self, file: Found, stopping: &AtomicBool) -> bool {
        let followed = self
            .followed
            .entry(file.path.clone())
            .or_insert_with(|| Followed::new(&file));
        let stamp: Stamp = file.stamp;
        if followed.offset > 0 && followed.found.stamp == stamp {
            return false;
        }
        if stamp.size < followed.offset {
            *followed = Followed::new(&file);
        }
        followed.found = file;
        if followed.verdict == Verdict::Excluded {
            return false;
        }
        let bounds = Bounds {
            home: &self.home,
            macos: self.macos,
        };
        let (settings, repositories) = (&self.settings, &mut self.repositories);
        let verdict = &mut followed.verdict;
        let mut halt = |tally: &Tally| {
            if *verdict == Verdict::Unknown
                && let Some(cwd) = tally.cwd()
            {
                *verdict = decide(cwd, settings, repositories, &bounds);
            }
            *verdict == Verdict::Excluded || stopping.load(Ordering::Relaxed)
        };
        let before = followed.offset;
        let read = lines::read_new(&followed.found.path, before, &mut followed.tally, &mut halt);
        if followed.verdict == Verdict::Excluded {
            followed.tally = Tally::new(followed.found.source);
            return false;
        }
        match read {
            Ok(offset) => {
                followed.offset = offset;
                offset != before
            }
            Err(error) => {
                tracing::debug!(kind = %error.kind(), "a session file could not be read");
                false
            }
        }
    }

    /// The session days of each session with a file in `changed`: a Claude Code session is
    /// named by its file, a Codex one by the root of its thread's chain.
    fn session_days(&self, changed: &HashSet<PathBuf>) -> Vec<SessionDay> {
        let parents: HashMap<&str, &str> = self
            .followed
            .values()
            .filter_map(|file| match file.tally.thread()? {
                (own, Some(parent)) => Some((own, parent)),
                (_, None) => None,
            })
            .collect();
        let mut sessions: BTreeMap<(Source, &str), Group> = BTreeMap::new();
        for (path, file) in &self.followed {
            if file.verdict == Verdict::Excluded {
                continue;
            }
            let id = match (&file.found.session, file.tally.thread()) {
                (Some(id), _) => id.as_str(),
                (None, Some((own, _))) => root_thread(own, |id| parents.get(id).copied()),
                (None, None) => continue,
            };
            let group = sessions.entry((file.found.source, id)).or_default();
            group.changed |= changed.contains(path);
            if group.cwd.is_none() || file.found.main {
                group.cwd = file.tally.cwd().or(group.cwd);
            }
            group.tallies.push(&file.tally);
        }
        sessions
            .into_iter()
            .filter(|(_, group)| group.changed)
            .flat_map(|((source, id), group)| {
                let name = group
                    .cwd
                    .and_then(|cwd| self.repositories.get(cwd))
                    .and_then(|repository| repository.name.clone());
                let mut session = Session::new(source, id, name);
                for tally in group.tallies {
                    session.add(tally);
                }
                session.session_days()
            })
            .collect()
    }
}

/// The files of one session.
#[derive(Default)]
struct Group<'a> {
    changed: bool,
    cwd: Option<&'a str>,
    tallies: Vec<&'a Tally>,
}

impl Followed {
    fn new(found: &Found) -> Self {
        Self {
            found: found.clone(),
            offset: 0,
            tally: Tally::new(found.source),
            verdict: Verdict::Unknown,
        }
    }
}

/// Whether a session in `cwd` may be counted: not when the folder, the main folder of its
/// repository or the repository is excluded. Remembers the repository of `cwd`.
fn decide(
    cwd: &str,
    settings: &UsageSettings,
    repositories: &mut HashMap<String, Repository>,
    bounds: &Bounds,
) -> Verdict {
    if repositories.len() >= MAX_REMEMBERED {
        repositories.clear();
    }
    let folder = Path::new(cwd);
    let repository = repositories
        .entry(cwd.to_owned())
        .or_insert_with(|| git::repository(folder, bounds));
    let mut folders = vec![folder];
    folders.extend(repository.main.as_deref());
    folders.extend(daemon_core::agent_usage::repository::claude_worktree_main(
        folder,
    ));
    if settings.excludes(&folders, repository.name.as_deref()) {
        Verdict::Excluded
    } else {
        Verdict::Counted
    }
}
