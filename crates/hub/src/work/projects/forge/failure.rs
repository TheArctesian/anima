//! A live read's failures, and what came of a change, as K7 codes with fixed words (K-E1
//! Section 13.7; decisions item 7). Never a reply of GitHub, a token, a path or text the
//! operator wrote (LOG-007). Pure.

use async_graphql::Error as GraphQlError;
use source_exchange::{Receipt, Refusal, Settled};

use crate::{
    api_errors::{ErrorCode, coded, internal},
    engine::sources::LiveFailure,
};

/// What a change is made to: the noun of its fixed words. The pull request changes add theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Issue,
    /// A comment goes on an issue or a pull request.
    IssueOrPullRequest,
    Run,
    Workflow,
    Commit,
    PullRequest,
}

impl Subject {
    const fn noun(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::IssueOrPullRequest => "issue or pull request",
            Self::Run => "run",
            Self::Workflow => "workflow",
            Self::Commit => "commit",
            Self::PullRequest => "pull request",
        }
    }

    /// The words of a 410. GitHub answers it on its issue paths when issues are off.
    const fn gone(self) -> &'static str {
        match self {
            Self::Issue | Self::IssueOrPullRequest => "Issues are off for this repository.",
            Self::Run | Self::Workflow | Self::Commit => {
                "GitHub says it is gone. Nothing was changed."
            }
            Self::PullRequest => "GitHub refused the merge.",
        }
    }
}

/// The words of a change that was sent and no whole answer came back (`Settled::Unknown`).
pub const UNKNOWN: &str =
    "GitHub did not answer. The change may have been made: look before you try again.";

/// "GitHub limits requests until HH:MM." The time is UTC, as every time the hub shows (K7).
#[must_use]
pub fn words(failure: LiveFailure) -> String {
    match failure {
        LiveFailure::Limited { until } => {
            format!("GitHub limits requests until {}.", until.format("%H:%M"))
        }
        LiveFailure::Unreachable => "GitHub could not be reached. Try again.".to_owned(),
        LiveFailure::Unusable => "GitHub's answer could not be used. Try again.".to_owned(),
        LiveFailure::Busy => {
            "The hub is still busy with this account's requests. Try again.".to_owned()
        }
        LiveFailure::KeyRefused => "GitHub refused the key. Connect the account again.".to_owned(),
        LiveFailure::Paused => {
            "Its account is paused. Resume it to read the repository.".to_owned()
        }
        LiveFailure::NotFound => {
            "This repository is not connected: no source account holds it.".to_owned()
        }
        LiveFailure::Refused | LiveFailure::NoChanges => {
            "The hub does not send this request to GitHub.".to_owned()
        }
        LiveFailure::Store => "The hub could not read the account.".to_owned(),
    }
}

/// A failed live read with nothing held: `UNAVAILABLE` while GitHub or the hub's turn is the
/// cause, `CONFLICT` when the account must change first, `INTERNAL` for the database.
#[must_use]
pub fn failed(failure: LiveFailure) -> GraphQlError {
    let code = match failure {
        LiveFailure::Store => return internal(failure),
        LiveFailure::Limited { .. }
        | LiveFailure::Unreachable
        | LiveFailure::Unusable
        | LiveFailure::Busy => ErrorCode::Unavailable,
        LiveFailure::KeyRefused
        | LiveFailure::Paused
        | LiveFailure::NotFound
        | LiveFailure::Refused
        | LiveFailure::NoChanges => ErrorCode::Conflict,
    };
    coded(words(failure), code)
}

/// What an answer of GitHub with `status` means for a read: `Ok(true)` for 200, `Ok(false)`
/// for 404 (GitHub has no such thing, or the App may not read it), and an error for the rest.
/// GitHub answers 410 only on its issue paths, when issues are off.
///
/// # Errors
/// `FORBIDDEN` for 403 (a limit never reaches here: the engine took it), `CONFLICT` for 410,
/// `UNAVAILABLE` for any other status.
pub fn answered(status: u16) -> Result<bool, GraphQlError> {
    match status {
        200 => Ok(true),
        404 => Ok(false),
        403 => Err(coded(
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            ErrorCode::Forbidden,
        )),
        410 => Err(coded(
            "Issues are off for this repository.",
            ErrorCode::Conflict,
        )),
        _ => Err(unusable()),
    }
}

/// `NOT_FOUND` for a 404 where the root has no null to give: `what` is a fixed noun.
#[must_use]
pub fn missing(what: &str) -> GraphQlError {
    coded(
        format!("GitHub has no such {what}, or the App may not read it."),
        ErrorCode::NotFound,
    )
}

/// `UNAVAILABLE`: GitHub answered something the hub cannot use.
#[must_use]
pub fn unusable() -> GraphQlError {
    coded(words(LiveFailure::Unusable), ErrorCode::Unavailable)
}

/// `UNAVAILABLE`: GitHub's answer is larger than the hub reads.
#[must_use]
pub fn too_large() -> GraphQlError {
    coded(
        "GitHub's answer is larger than the hub reads.",
        ErrorCode::Unavailable,
    )
}

/// `CONFLICT`: the stored name cannot go into a request (the provider refused it).
#[must_use]
pub fn unnamed() -> GraphQlError {
    coded(
        "The hub cannot read a repository with this name on GitHub.",
        ErrorCode::Conflict,
    )
}

/// A change the engine did not send: as [`failed`], but a connection that failed says that
/// nothing was changed.
#[must_use]
pub fn unsent(failure: LiveFailure) -> GraphQlError {
    match failure {
        LiveFailure::Unreachable => coded(
            "GitHub could not be reached. Nothing was changed.",
            ErrorCode::Unavailable,
        ),
        other => failed(other),
    }
}

/// What came of a change that was sent to `subject`: the receipt when GitHub made it.
///
/// # Errors
/// The K7 code and fixed words of a refusal, a limit (`UNAVAILABLE`, until when) or an answer
/// that does not say whether the change was made (`UNAVAILABLE`, [`UNKNOWN`]).
pub fn settled(settled: Settled, subject: Subject) -> Result<Receipt, GraphQlError> {
    match settled {
        Settled::Done(receipt) => Ok(receipt),
        Settled::Refused(refusal) => Err(refused(refusal, subject)),
        Settled::Limited { until } => Err(coded(
            words(LiveFailure::Limited { until }),
            ErrorCode::Unavailable,
        )),
        Settled::Unknown => Err(coded(UNKNOWN, ErrorCode::Unavailable)),
    }
}

/// GitHub refused a change to `subject`: nothing was changed. A 404 can mean that the App may
/// not change a private repository (decisions item 7).
#[must_use]
pub fn refused(refusal: Refusal, subject: Subject) -> GraphQlError {
    let (words, code) = match refusal {
        Refusal::KeyRefused => (words(LiveFailure::KeyRefused), ErrorCode::Conflict),
        Refusal::NotPermitted => (
            "The GitHub App is not permitted to do that. Check its permissions on GitHub."
                .to_owned(),
            ErrorCode::Forbidden,
        ),
        Refusal::NotFound => (
            format!(
                "GitHub has no such {}, or the App may not change it.",
                subject.noun()
            ),
            ErrorCode::NotFound,
        ),
        Refusal::HeadMoved => (
            "The pull request changed since you looked. Read it again.".to_owned(),
            ErrorCode::Conflict,
        ),
        Refusal::NotMergeable => ("GitHub refused the merge.".to_owned(), ErrorCode::Conflict),
        Refusal::Invalid => (
            "GitHub refused a value of the change. Nothing was changed.".to_owned(),
            ErrorCode::Validation,
        ),
        Refusal::Gone => (subject.gone().to_owned(), ErrorCode::Conflict),
        Refusal::Conflict => (
            format!(
                "GitHub refused it: the {} is not in a state that allows this. Read it again.",
                subject.noun()
            ),
            ErrorCode::Conflict,
        ),
        Refusal::Unexplained => (
            "GitHub refused the change. Nothing was changed.".to_owned(),
            ErrorCode::Unavailable,
        ),
    };
    coded(words, code)
}

/// `INTERNAL` once GitHub made the change and the hub could not record it, twice (K-E1
/// Section 13.6). `cause` keeps its source for the process log.
#[must_use]
pub fn unrecorded(mut cause: GraphQlError) -> GraphQlError {
    "GitHub made the change, but the hub could not record it.".clone_into(&mut cause.message);
    cause
}

/// `UNAVAILABLE`: GitHub made the change, but the hub could not read the result back.
#[must_use]
pub fn unread() -> GraphQlError {
    coded(
        "GitHub made the change, but the hub could not read it back. Read it again.",
        ErrorCode::Unavailable,
    )
}
