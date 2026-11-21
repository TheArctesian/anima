//! The use of the AI agent programs on a device (AIS-018 to AIS-024, K8 Section 4): counts of
//! prompts, tool calls and tokens of each session, by UTC day and by model.
//!
//! Pure: [`Tally`] reads the lines of one session file of Claude Code or Codex and keeps only
//! the counts, the times, the working folder and the IDs that link a sub-agent to its session;
//! never the text of a line (AIS-018, AIS-022). [`Session`] adds the tallies of one session's
//! files and gives its [`SessionDay`]s. [`repository`] reads the name of a repository from a git
//! configuration. The daemon reads the files (I/O); the hub checks what arrives
//! (`wire::BatchAgentUsage`).

pub mod account;
mod claude;
mod codex;
mod content;
mod names;
pub mod repository;
mod session;
mod tally;
#[cfg(test)]
mod tests;

pub use codex::root_thread;
pub use names::{is_model, is_repository, is_session_id, is_word, model_or_other};
pub use session::Session;
pub use tally::{Day, Tally};

use chrono::{DateTime, NaiveDate, Utc};

/// The most session days one batch may hold (K8 Section 4).
pub const MAX_SESSION_DAYS: usize = 16;

/// The most models one session day may hold (K8 Section 4).
pub const MAX_MODELS: usize = 16;

/// The largest count a batch may carry: 2^53 − 1, exact in every JSON reader and in a
/// PostgreSQL `bigint` (K8 Section 4).
pub const MAX_COUNT: u64 = (1 << 53) - 1;

/// The model name sent for a model whose name breaks the K8 rule, and for the smallest models
/// of a day that has more than [`MAX_MODELS`].
pub const OTHER_MODEL: &str = "other";

/// The AI agent program a session comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    ClaudeCode,
    Codex,
}

impl Source {
    /// The source of a K8 `source` text; `None` for one this side does not know.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "claude_code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude_code",
            Self::Codex => "codex",
        }
    }
}

/// What a daemon can read of the sessions on its computer (AIS-024, K8 Section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UsageState {
    /// It reads the session files of its user.
    Reading,
    /// It runs as the systemd system unit, which cannot see the files of a user.
    SystemUnit,
    /// The operator turned the function off, or its settings cannot be read.
    TurnedOff,
}

impl UsageState {
    /// The state of a K8 `state` text; `None` for one this side does not know.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "reading" => Some(Self::Reading),
            "system_unit" => Some(Self::SystemUnit),
            "turned_off" => Some(Self::TurnedOff),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::SystemUnit => "system_unit",
            Self::TurnedOff => "turned_off",
        }
    }
}

/// The tokens of one model: the input not read from or written to the cache, the output
/// (reasoning included), and the input read from and written to the cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Tokens {
    /// These tokens and `more`, each count held at [`MAX_COUNT`].
    #[must_use]
    pub fn plus(self, more: Self) -> Self {
        let add = |a: u64, b: u64| a.saturating_add(b).min(MAX_COUNT);
        Self {
            input: add(self.input, more.input),
            output: add(self.output, more.output),
            cache_read: add(self.cache_read, more.cache_read),
            cache_write: add(self.cache_write, more.cache_write),
        }
    }

    /// Whether each count is at most [`MAX_COUNT`].
    #[must_use]
    pub const fn fits(self) -> bool {
        self.input <= MAX_COUNT
            && self.output <= MAX_COUNT
            && self.cache_read <= MAX_COUNT
            && self.cache_write <= MAX_COUNT
    }
}

/// The tokens of one model on one session day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelTokens {
    pub model: String,
    pub tokens: Tokens,
}

/// The counts of one session on one UTC day (K8 Section 4): the unit the hub upserts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionDay {
    pub source: Source,
    /// The program's session ID; a sub-agent counts under the session that started it.
    pub id: String,
    pub day: NaiveDate,
    /// The first and last time of the whole session, all its days.
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    /// `host/owner/name`, see [`repository`].
    pub repository: Option<String>,
    pub prompts: u64,
    pub tool_calls: u64,
    /// Unique by name, at most [`MAX_MODELS`].
    pub models: Vec<ModelTokens>,
}

/// What the hub takes from an `agentUsage` member: the state when it knows it, and the session
/// days of the sources it knows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentUsage {
    pub state: Option<UsageState>,
    pub session_days: Vec<SessionDay>,
    /// The accounts of the known sources; `None` when the batch sent no `accounts` member, which
    /// keeps what the hub has (AIS-022, R-NEW-accountgroup).
    pub accounts: Option<Vec<account::Account>>,
}
