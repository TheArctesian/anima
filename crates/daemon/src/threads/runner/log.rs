//! The log of the runner at the machine (THR-044): a file of the user, mode 0600, with a limit
//! of size, with five kinds of line. It holds no text of a prompt and no output. It is a record
//! for the person, not a guard: a program of the same user can change it.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// The name of the log in the folder of the threads.
pub const FILE: &str = "runner.log";

/// The most bytes of the log; past it the log moves to `runner.log.1` and starts again.
pub const LIMIT: u64 = 1024 * 1024;

/// The five kinds of line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Start,
    Prompt,
    Answer,
    Stop,
    Refusal,
}

/// One line: the time, the kind, the thread, what the runner could see of the source, and a
/// short word of what happened (never text of the person or of the program).
#[derive(Clone, Debug, Serialize)]
pub struct Line<'a> {
    pub at: DateTime<Utc>,
    pub kind: Kind,
    pub thread: Option<Uuid>,
    pub source: &'a str,
    pub what: &'a str,
}

/// The log file of one runner.
#[derive(Debug)]
pub struct RunnerLog {
    path: PathBuf,
    limit: u64,
}

impl RunnerLog {
    #[must_use]
    pub fn at(dir: &Path) -> Self {
        Self {
            path: dir.join(FILE),
            limit: LIMIT,
        }
    }

    /// Adds one line. A fault of the file is returned to the caller, which says it once.
    ///
    /// # Errors
    /// The I/O error of the file.
    pub fn write(&self, line: &Line<'_>) -> std::io::Result<()> {
        use std::os::unix::fs::OpenOptionsExt as _;
        if std::fs::metadata(&self.path).is_ok_and(|meta| meta.len() >= self.limit) {
            std::fs::rename(&self.path, self.path.with_extension("log.1"))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&self.path)?;
        let mut text = serde_json::to_string(line).map_err(std::io::Error::other)?;
        text.push('\n');
        file.write_all(text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_044_the_log_is_private_has_a_limit_and_holds_no_prompt() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("a folder");
        let log = RunnerLog {
            limit: 1000,
            ..RunnerLog::at(dir.path())
        };
        let line = Line {
            at: Utc::now(),
            kind: Kind::Prompt,
            thread: Some(Uuid::nil()),
            source: "local socket",
            what: "sent",
        };
        let size = serde_json::to_string(&line).expect("a line").len() as u64 + 1;
        for _ in 0..=(2 * log.limit / size) {
            log.write(&line).expect("written");
        }
        let path = dir.path().join(FILE);
        let mode = std::fs::metadata(&path)
            .expect("there")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        assert!(std::fs::metadata(&path).expect("there").len() < log.limit + size);
        assert!(dir.path().join("runner.log.1").exists());
        let text = std::fs::read_to_string(path).expect("reads");
        assert!(text.contains(r#""kind":"prompt""#));
    }
}
