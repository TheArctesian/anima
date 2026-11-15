//! What the caller gives to merge, checked into parts before the hub reads or sends anything
//! (K1 `MergePullRequestInput`, `PullRequestTargetInput`). A refusal names the field, never its
//! value (LOG-007). Pure.

use async_graphql::{ID, InputObject, Result};
use source_exchange::{CommitSha, Number};

use super::views::MergeMethod;
use crate::{
    api_errors::{FieldError, invalid},
    ids::RepositoryId,
};

/// The most pull requests one bulk merge takes (K1 `mergePullRequests`).
pub const MOST: usize = 20;

#[derive(InputObject)]
#[graphql(name = "MergePullRequestInput")]
pub struct MergePullRequestInput {
    pub repository: ID,
    pub number: i32,
    /// The head commit the operator saw, 40 hex digits. CONFLICT when the head moved since.
    pub expected_head_sha: String,
    pub method: MergeMethod,
    /// True also merges an UNSTABLE pull request (a check that is not required failed). BLOCKED is never merged.
    #[graphql(default_with = "Some(false)")]
    pub allow_failing_checks: Option<bool>,
    /// Required for a pull request from a fork: must equal its headRepository.
    pub fork_confirmed: Option<String>,
}

#[derive(InputObject)]
#[graphql(name = "PullRequestTargetInput")]
pub struct PullRequestTargetInput {
    pub repository: ID,
    pub number: i32,
    pub expected_head_sha: String,
}

/// A pull request as the caller saw it: its number and its head commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub number: Number,
    pub head: CommitSha,
}

/// One pull request of a bulk merge, checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// As the caller gave it, for the result.
    pub repository: ID,
    pub seen: Seen,
}

/// The pull request `input` names, as the caller saw it.
///
/// # Errors
/// One `VALIDATION` error listing `number` and `expectedHeadSha` when either is not one.
pub fn one(input: &MergePullRequestInput) -> Result<Seen> {
    match (number(input.number), head(&input.expected_head_sha)) {
        (Ok(number), Ok(head)) => Ok(Seen { number, head }),
        (number, head) => Err(invalid(
            &number
                .err()
                .into_iter()
                .chain(head.err())
                .collect::<Vec<_>>(),
        )),
    }
}

/// The pull requests of a bulk merge, in the order given.
///
/// # Errors
/// `VALIDATION` naming `pullRequests` for none or more than [`MOST`], an entry that is not one,
/// or a pull request given twice.
pub fn many(list: &[PullRequestTargetInput]) -> Result<Vec<Entry>> {
    if list.is_empty() || list.len() > MOST {
        return Err(refused("Give 1 to 20 pull requests."));
    }
    let mut entries: Vec<(RepositoryId, Entry)> = Vec::with_capacity(list.len());
    for given in list {
        let checked = (
            given.repository.parse::<RepositoryId>(),
            number(given.number),
            head(&given.expected_head_sha),
        );
        let (Ok(id), Ok(number), Ok(head)) = checked else {
            return Err(refused(
                "Give each pull request a repository id, a number of 1 or more and the head \
                 commit you saw, 40 hex digits.",
            ));
        };
        if entries
            .iter()
            .any(|(seen_id, entry)| *seen_id == id && entry.seen.number == number)
        {
            return Err(refused("Give each pull request once."));
        }
        let seen = Seen { number, head };
        let repository = given.repository.clone();
        entries.push((id, Entry { repository, seen }));
    }
    Ok(entries.into_iter().map(|(_, entry)| entry).collect())
}

fn refused(words: &'static str) -> async_graphql::Error {
    invalid(&[FieldError::new("pullRequests", words)])
}

fn number(number: i32) -> Result<Number, FieldError> {
    u64::try_from(number)
        .ok()
        .and_then(|n| Number::new(n).ok())
        .ok_or_else(|| FieldError::new("number", "Give a number of 1 or more."))
}

fn head(sha: &str) -> Result<CommitSha, FieldError> {
    CommitSha::parse(sha).map_err(|_| {
        FieldError::new(
            "expectedHeadSha",
            "Give the head commit you saw: 40 hex digits.",
        )
    })
}
