//! The command line of an agent program (THR-020, THR-021, THR-040; the closed list of flags of
//! the TH18 record, Section 4).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DriveError;

/// The agent program of a thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Program {
    ClaudeCode,
    Codex,
}

/// The permission mode of a thread (THR-033: Full access is the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The agent runs each command with no question (measured in TH18).
    #[default]
    FullAccess,
    /// File changes in the folder need no question; commands ask.
    AcceptEdits,
    /// Each tool call that the program does not hold for safe asks.
    Ask,
}

impl Mode {
    /// The value of `--permission-mode` of Claude Code.
    #[must_use]
    pub fn claude(self) -> &'static str {
        match self {
            Self::FullAccess => "bypassPermissions",
            Self::AcceptEdits => "acceptEdits",
            Self::Ask => "manual",
        }
    }

    /// The `sandbox` and the `approvalPolicy` of a Codex thread. `AcceptEdits` has no measured
    /// Codex form; it is the sandbox of the folder with questions when the agent asks.
    #[must_use]
    pub fn codex(self) -> (&'static str, &'static str) {
        match self {
            Self::FullAccess => ("danger-full-access", "never"),
            Self::AcceptEdits => ("workspace-write", "on-request"),
            Self::Ask => ("workspace-write", "untrusted"),
        }
    }
}

/// A model name the person chose: an alias or a full name, of a closed alphabet (THR-040).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Model(String);

impl Model {
    /// # Errors
    /// [`DriveError::Refused`] when the name is empty, longer than 64 bytes, starts with `-`
    /// or holds a byte other than a letter, a digit, `.`, `_`, `-` or `[`, `]` (`[1m]`).
    pub fn new(name: &str) -> Result<Self, DriveError> {
        let fits = !name.is_empty()
            && name.len() <= 64
            && !name.starts_with('-')
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-[]".contains(&b));
        if fits {
            Ok(Self(name.to_owned()))
        } else {
            Err(DriveError::Refused { what: "model" })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The effort of the model, from the closed list of `claude --help`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

/// How a Claude Code process starts its session. The ID comes from the record of the runner,
/// never from a client (THR-040); a [`Uuid`] cannot hold a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStart {
    New(Uuid),
    Resume(Uuid),
    Fork(Uuid),
}

/// What the runner needs to start one agent program for one thread.
#[derive(Clone, Debug)]
pub struct Launch {
    pub program: Program,
    /// The absolute path of the program, found at the machine.
    pub path: PathBuf,
    pub mode: Mode,
    pub model: Option<Model>,
    pub effort: Option<Effort>,
    /// Claude Code only; a Codex thread is named on the pipe.
    pub session: SessionStart,
}

/// A program and its arguments, for a start with no shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Launch {
    /// The command line. Claude Code gets only the flags of the closed list of the TH18 record;
    /// never `--settings`, `--setting-sources`, `--allowedTools`, `--add-dir` or an MCP flag.
    /// Codex gets `app-server` and nothing else: its mode goes on `thread/start`.
    ///
    /// # Errors
    /// [`DriveError::RelativeProgram`] when the path of the program is not absolute.
    pub fn invocation(&self) -> Result<Invocation, DriveError> {
        if !Path::new(&self.path).is_absolute() {
            return Err(DriveError::RelativeProgram);
        }
        let args = match self.program {
            Program::Codex => vec!["app-server".to_owned()],
            Program::ClaudeCode => self.claude_args(),
        };
        Ok(Invocation {
            program: self.path.clone(),
            args,
        })
    }

    fn claude_args(&self) -> Vec<String> {
        let mut args: Vec<String> = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--permission-prompt-tool",
            "stdio",
            "--permission-mode",
            self.mode.claude(),
        ]
        .map(str::to_owned)
        .to_vec();
        if let Some(model) = &self.model {
            args.extend(["--model".to_owned(), model.as_str().to_owned()]);
        }
        if let Some(effort) = self.effort {
            args.extend(["--effort".to_owned(), effort.as_str().to_owned()]);
        }
        match self.session {
            SessionStart::New(id) => args.extend(["--session-id".to_owned(), id.to_string()]),
            SessionStart::Resume(id) => args.extend(["--resume".to_owned(), id.to_string()]),
            SessionStart::Fork(id) => args.extend([
                "--resume".to_owned(),
                id.to_string(),
                "--fork-session".to_owned(),
            ]),
        }
        args
    }
}
