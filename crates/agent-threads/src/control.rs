//! The local socket between a client of this computer (the `anima` command, the desktop shell)
//! and the thread runner of the daemon (TH12, THR-128): one line of JSON for each frame.
//!
//! Pure: the frames and the place of the socket. The runner and the clients do the I/O.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::drive::{Answer, DriveEvent, Effort, Mode, Program, Request};

/// The most bytes of one frame of a client.
pub const MAX_FRAME: usize = 4 * 1024 * 1024;

/// The name of the socket file in the folder of the threads of the daemon.
pub const SOCKET: &str = "runner.sock";

/// The folder of the threads of the daemon: `threads` below the folder of the daemon, which is
/// `ANIMA_DAEMON_HOME`, else on macOS `~/Library/Application Support/anima-daemon`, else
/// `$XDG_CONFIG_HOME/anima-daemon` or `~/.config/anima-daemon` (as `Home::locate` of the daemon).
#[must_use]
pub fn folder(env: impl Fn(&str) -> Option<String>, macos: bool) -> Option<PathBuf> {
    let daemon = if let Some(set) = env("ANIMA_DAEMON_HOME") {
        PathBuf::from(set)
    } else if macos {
        Path::new(&env("HOME")?).join("Library/Application Support/anima-daemon")
    } else if let Some(config) = env("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
        Path::new(&config).join("anima-daemon")
    } else {
        Path::new(&env("HOME")?).join(".config/anima-daemon")
    };
    Some(daemon.join("threads"))
}

/// A call of a client, with an ID that the client chose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub id: u64,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "call", rename_all = "snake_case")]
pub enum Action {
    /// The threads that the runner started, with their state.
    List,
    /// The updates of each thread from now on, after the updates that the runner keeps of it.
    Watch,
    /// A new thread in a folder that the person chose at this computer (THR-041).
    Start {
        program: Program,
        folder: PathBuf,
        #[serde(default)]
        mode: Mode,
        model: Option<String>,
        effort: Option<Effort>,
        prompt: String,
    },
    /// A prompt to a thread: to its running turn, or as a new turn.
    Send { thread: Uuid, text: String },
    /// The answer of the person to a request of the program (THR-030).
    Answer {
        thread: Uuid,
        request: Uuid,
        answer: Answer,
    },
    /// Stops the running turn; the program stays (THR-049).
    Interrupt { thread: Uuid },
    /// Ends the program of the thread and each process below it (THR-049).
    Stop { thread: Uuid },
    /// The person raised the soft limit (THR-114).
    RaiseLimit,
    /// What the project files of a folder make the program do with no question, to show before
    /// a thread starts there (THR-046).
    Folder { folder: PathBuf },
}

/// A frame of the runner to a client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum Reply {
    Done { id: u64, result: Outcome },
    Update(Box<Update>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Ok,
    Started {
        thread: Uuid,
    },
    Threads {
        threads: Vec<ThreadRow>,
    },
    Limits {
        limits: Limits,
    },
    /// The notice of a folder, as lines for the person; empty when there is nothing to say.
    Folder {
        notice: Vec<String>,
    },
    /// The program of a stopped thread ended; `all_ended` is false when a process below it was
    /// still there or the walk could not run (THR-049).
    Stopped {
        all_ended: bool,
    },
    Refused {
        why: Refusal,
    },
}

/// Why the runner did not do a call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "refusal", rename_all = "snake_case")]
pub enum Refusal {
    UnknownThread,
    /// The folder is not an absolute folder of this computer (THR-041).
    FolderNotPermitted,
    /// The program is not on the `PATH` of the login of the person.
    ProgramNotFound,
    /// A value that becomes an argument is not of its alphabet (THR-040).
    BadValue,
    /// The request is answered, or its process ended (THR-030).
    RequestGone,
    /// The answer does not fit the request.
    AnswerDoesNotFit,
    /// The thread is not running.
    NotRunning,
    /// The holder did not take the command.
    Holder,
    /// A start would pass the soft limit: the person must answer first (THR-114).
    OverLimit {
        limits: Limits,
    },
}

/// The soft limits of the threads of this computer (THR-114): guidance, raised by the person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub depth: u32,
    pub children: u32,
    pub running: u32,
}

/// The words of the operator at a limit (THR-114, R-308).
pub const OVER_LIMIT: &str = "You are over the recommended guidance and at risk of cyberpsychosis from running this many sessions. Do you want to raise the limit?";

impl Limits {
    pub const FIRST: Self = Self {
        depth: 3,
        children: 5,
        running: 10,
    };

    /// The next step: 5, 10, 15 after the first; then each step doubles the one before it, with
    /// no end: 10, 20, 30, then 20, 40, 60 (THR-114, the answer of the operator "Keep doubling").
    #[must_use]
    pub const fn raised(self) -> Self {
        if self.depth == Self::FIRST.depth
            && self.children == Self::FIRST.children
            && self.running == Self::FIRST.running
        {
            return Self {
                depth: 5,
                children: 10,
                running: 15,
            };
        }
        Self {
            depth: self.depth.saturating_mul(2),
            children: self.children.saturating_mul(2),
            running: self.running.saturating_mul(2),
        }
    }
}

/// What a thread does now, by the order of trust of THR-085: the end of the process first, then
/// the stream of the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Starting,
    Working,
    /// A request waits for the person; only an answer, a stop or the end of the process clears
    /// it (THR-086, THR-087).
    NeedsYou,
    Idle,
    /// The program ended; `Stopped` tells why in [`ThreadRow::ended`].
    Stopped,
    /// No source can tell the state (THR-086).
    Unknown,
}

/// One thread of the runner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadRow {
    pub id: Uuid,
    pub program: Program,
    pub folder: PathBuf,
    pub mode: Mode,
    pub model: Option<String>,
    pub state: RunState,
    /// The session ID of Claude Code or the thread ID of Codex, when the program named it.
    pub session: Option<String>,
    pub title: String,
    pub started: chrono::DateTime<chrono::Utc>,
    /// Why the thread stopped, in words for the person.
    pub ended: Option<String>,
    /// The requests that wait for the person.
    pub open: Vec<Uuid>,
}

/// One update of one thread, in the order the runner made them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Update {
    pub thread: Uuid,
    pub seq: u64,
    #[serde(flatten)]
    pub change: Change,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum Change {
    /// Something the program said. A request of the program comes as [`Change::Asked`].
    Said {
        event: DriveEvent,
    },
    /// The program asks the person; `request` is the ID of the runner for the answer.
    Asked {
        request: Uuid,
        detail: Request,
    },
    /// A request is closed: answered, or gone with its process or turn.
    Closed {
        request: Uuid,
        why: String,
    },
    /// A prompt the runner sent, with where it came from (THR-042).
    Prompt {
        text: String,
        source: String,
    },
    State {
        state: RunState,
        row: ThreadRow,
    },
}

#[cfg(test)]
mod limit_tests {
    use super::Limits;

    #[test]
    fn thr_114_the_soft_limits_go_to_5_10_15_then_double_with_no_end() {
        let steps: Vec<(u32, u32, u32)> =
            std::iter::successors(Some(Limits::FIRST), |limits| Some(limits.raised()))
                .take(5)
                .map(|limits| (limits.depth, limits.children, limits.running))
                .collect();
        assert_eq!(
            steps,
            [
                (3, 5, 10),
                (5, 10, 15),
                (10, 20, 30),
                (20, 40, 60),
                (40, 80, 120)
            ]
        );
        let top = Limits {
            depth: u32::MAX,
            children: u32::MAX,
            running: u32::MAX,
        };
        assert_eq!(top.raised(), top, "no overflow at the end of the numbers");
    }
}
