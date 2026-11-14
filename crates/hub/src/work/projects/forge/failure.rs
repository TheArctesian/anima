//! A live read's failures and GitHub's refusals as K7 codes with fixed words (K-E1 Section
//! 13.7). Never a reply of GitHub, a token or a path (LOG-007). Pure.

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal},
    engine::sources::LiveFailure,
};

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
