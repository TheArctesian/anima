//! Why a command failed, as the exit code (CMD-021), the code and the hint that `--json` prints
//! (CMD-022). Every command returns this; `main` turns it into the exit status.

use crate::auth::token_store::StoreError;
use crate::contexts::config::ConfigError;
use crate::hub::{Code, FieldError, HubError};

/// What to do next, as CMD-022 shows it: a command to run, or a sentence. A sentence is never
/// printed as if it were a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hint {
    /// A command the operator can paste.
    Run(String),
    /// Advice in words.
    Say(String),
}

impl Hint {
    #[must_use]
    pub fn run(command: impl Into<String>) -> Self {
        Self::Run(command.into())
    }

    #[must_use]
    pub fn say(sentence: impl Into<String>) -> Self {
        Self::Say(sentence.into())
    }

    /// The hint's words, as `--json` carries them.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Run(text) | Self::Say(text) => text,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error(transparent)]
    Hub(#[from] HubError),
    /// Wrong usage: a missing value under `--no-input`, two inputs that exclude each other.
    #[error("{message}")]
    Usage { message: String, hint: Option<Hint> },
    /// Something the operator named is not there: a context, a device name.
    #[error("{message}")]
    NotFound { message: String, hint: Option<Hint> },
    /// The hub or the device refused for a reason that has its own next step, not the token.
    #[error("{message}")]
    Forbidden { message: String, hint: Option<Hint> },
    /// A notification the hub sent for a test did not arrive.
    #[error("{message}")]
    NotDelivered { message: String },
    /// The operator was asked to confirm and said no: nothing was changed.
    #[error("Nothing was changed: you did not confirm.")]
    Declined,
    /// The operator pressed Ctrl-C at a question: nothing was changed.
    #[error("Nothing was changed: the question was cancelled.")]
    Cancelled,
    /// Something with this name is already there.
    #[error("{message}")]
    Conflict { message: String },
    /// No token for this hub: none stored and no `ANIMA_TOKEN`.
    #[error("You are not signed in to {origin}.")]
    SignedOut { origin: String },
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Reading a document, variables or a token from a file or standard input.
    #[error("Could not read {what}: {reason}.")]
    Input { what: String, reason: String },
    /// Paging with `--all` stopped: the hub repeated a cursor or the list passed the row cap.
    #[error("Stopped paging: {reason}.")]
    Paging { reason: String },
    /// Standard output failed (not a closed pipe).
    #[error("Could not write the output: {reason}.")]
    Write { reason: String },
}

/// CMD-021.
pub mod exit {
    pub const FAILURE: u8 = 1;
    pub const USAGE: u8 = 2;
    pub const NOT_FOUND: u8 = 3;
    pub const SIGN_IN: u8 = 4;
    pub const NOT_PERMITTED: u8 = 5;
    pub const CONFLICT: u8 = 6;
    pub const UNAVAILABLE: u8 = 7;
}

impl Failure {
    /// A usage failure that names the flag that would have given the missing value (CMD-023).
    #[must_use]
    pub fn missing(what: &str, flag: &str) -> Self {
        Self::Usage {
            message: format!(
                "{what} is needed and --no-input is set (or no terminal): pass {flag}."
            ),
            hint: None,
        }
    }

    /// The code for `--json` (CMD-022): the K7 code, or `USAGE` and `FAILED` for failures the
    /// hub has no code for.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Hub(error) => error.code().as_str(),
            Self::Usage { .. } => "USAGE",
            Self::Declined | Self::Cancelled => "DECLINED",
            Self::Forbidden { .. } => Code::Forbidden.as_str(),
            Self::NotFound { .. } => Code::NotFound.as_str(),
            Self::Conflict { .. } => Code::Conflict.as_str(),
            Self::SignedOut { .. } => Code::Unauthenticated.as_str(),
            Self::Config(_)
            | Self::Store(_)
            | Self::Input { .. }
            | Self::Write { .. }
            | Self::Paging { .. }
            | Self::NotDelivered { .. } => "FAILED",
        }
    }

    /// The process exit code (CMD-021).
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Hub(error) => match error.code() {
                Code::Unauthenticated => exit::SIGN_IN,
                Code::Forbidden => exit::NOT_PERMITTED,
                Code::NotFound => exit::NOT_FOUND,
                Code::Validation => exit::USAGE,
                Code::Conflict => exit::CONFLICT,
                Code::Unavailable => exit::UNAVAILABLE,
                Code::Internal => exit::FAILURE,
            },
            Self::Usage { .. } => exit::USAGE,
            Self::Forbidden { .. } => exit::NOT_PERMITTED,
            Self::NotFound { .. } => exit::NOT_FOUND,
            Self::Conflict { .. } => exit::CONFLICT,
            Self::SignedOut { .. } => exit::SIGN_IN,
            Self::Declined
            | Self::Cancelled
            | Self::Config(_)
            | Self::Store(_)
            | Self::Input { .. }
            | Self::Write { .. }
            | Self::Paging { .. }
            | Self::NotDelivered { .. } => exit::FAILURE,
        }
    }

    /// The next command to try (CMD-022).
    #[must_use]
    pub fn hint(&self) -> Option<Hint> {
        match self {
            Self::Usage { hint, .. }
            | Self::NotFound { hint, .. }
            | Self::Forbidden { hint, .. } => hint.clone(),
            Self::SignedOut { .. } => Some(Hint::run(SIGN_IN)),
            Self::Hub(error) => match error.code() {
                Code::Unauthenticated => Some(Hint::run(SIGN_IN)),
                Code::Forbidden => Some(Hint::say(
                    "An API key cannot do everything: check it with `anima auth status` or sign in with `anima auth login`.",
                )),
                Code::Unavailable => Some(Hint::run("anima auth status")),
                _ => None,
            },
            Self::Conflict { .. }
            | Self::Declined
            | Self::Cancelled
            | Self::NotDelivered { .. }
            | Self::Config(_)
            | Self::Store(_)
            | Self::Input { .. }
            | Self::Write { .. }
            | Self::Paging { .. } => None,
        }
    }

    /// The rejected input fields of a hub `VALIDATION` error.
    #[must_use]
    pub fn fields(&self) -> &[FieldError] {
        match self {
            Self::Hub(error) => error.fields(),
            _ => &[],
        }
    }
}

const SIGN_IN: &str = "anima auth login";
