//! The checked parts a named change is built from (PRJ-027 to PRJ-035, OD-E14).
//!
//! Every part has a private field and a checking constructor, so a [`crate::Change`] can only
//! hold values that are safe where a provider puts them: a part that becomes a path segment is
//! letters, digits, `.`, `-` and `_` only, never `.` or `..`; text is bounded and holds no NUL.
//! The errors are fixed words and never quote the refused value (LOG-007).

mod names;
mod numbers;
mod text;

pub use names::{BranchName, RepositoryName, StatusContext, TargetUrl, WorkflowId};
pub use numbers::{CommitSha, Number, RunId};
pub use text::{InputName, InputValue, Inputs, Label, Labels, StatusDescription, Text, Title};

/// A value that cannot be part of a change. Fixed words: the refused value is never quoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PartError {
    #[error("a repository is owner/name: letters, digits, dots, dashes and underscores")]
    Repository,
    #[error("a number is a whole number from 1 to 2^53")]
    Number,
    #[error("a commit is 40 hexadecimal digits")]
    Sha,
    #[error("a workflow is its number or its file name ending .yml or .yaml")]
    Workflow,
    #[error("the branch name is not one git allows")]
    Branch,
    #[error("a status context is anima/ then 1 to 40 of a-z, 0-9 and -")]
    StatusContext,
    #[error("the address must be below the hub's public origin")]
    TargetUrl,
    #[error("a title is 1 to 256 characters on one line")]
    Title,
    #[error("the text is 1 to 65,536 characters with no NUL or control character")]
    Text,
    #[error("a status description is at most 140 characters on one line")]
    StatusDescription,
    #[error("a label is 1 to 50 characters on one line, and there are at most 20")]
    Label,
    #[error("an input name is 1 to 100 of letters, digits, - and _")]
    InputName,
    #[error("an input value is at most 1,024 characters with no NUL or control character")]
    InputValue,
    #[error("at most 25 inputs, each name once")]
    Inputs,
}

/// How a pull request is merged. The caller always names it (no default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

/// What an issue becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueState {
    Open,
    /// `None`: the provider's default reason.
    Closed(Option<CloseReason>),
}

/// Why an issue is closed. A provider may read more reasons; these are the ones Anima writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Completed,
    NotPlanned,
}

/// The state a commit status reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusState {
    Pending,
    Success,
    Failure,
    Error,
}

/// Whether every byte is a letter, a digit, `.`, `-` or `_`, and the text is not empty, `.`
/// or `..`: such text is one path segment that a URL parser leaves as it is.
pub(crate) const fn is_plain_segment(text: &[u8]) -> bool {
    if text.is_empty() || matches!(text, [b'.'] | [b'.', b'.']) {
        return false;
    }
    let mut at = 0;
    while at < text.len() {
        let b = text[at];
        if !(b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_') {
            return false;
        }
        at += 1;
    }
    true
}
