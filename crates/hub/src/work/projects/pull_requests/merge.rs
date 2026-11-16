//! The merge rule (K-E1 Section 13.3; PRJ-033, PRO-013): from a pull request as GitHub just
//! gave it, the merge methods the repository allows and what the caller asked, either the merge
//! to send or one refusal. Pure: no read, no clock, no log.
//!
//! The rule never guesses: a mergeability GitHub has not worked out, or merge methods the hub
//! cannot see, refuse. A pull request from a fork is a trust decision by a person: it merges
//! only alone, only when the caller typed its head repository, and never for a request that
//! declares an AI agent.

use provider_github::live::pulls::{PullDetail, PullState};

use super::{
    bulk::MergeOutcome,
    views::{MergeMethod, MergeState, merge_state},
};
use crate::api_errors::ErrorCode;

/// How the caller merges: one pull request with its own choices, or one of many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merging<'a> {
    Alone {
        /// Also merge an UNSTABLE pull request.
        allow_failing_checks: bool,
        /// What the caller typed for a pull request from a fork.
        fork_confirmed: Option<&'a str>,
    },
    InBulk,
}

/// What the caller asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ask<'a> {
    pub method: MergeMethod,
    /// The head commit the caller saw, 40 hexadecimal digits.
    pub expected_head: &'a str,
    /// The request declares an AI agent (LOG-016, LOG-017).
    pub agent: bool,
    pub merging: Merging<'a>,
}

/// What the hub does with the pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Send the merge, with the head the caller saw.
    Merge,
    /// It was merged before: nothing to send.
    AlreadyMerged,
    Refused(Refusal),
}

/// Why the hub sends nothing. Each has fixed words, a K7 code and a bulk outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Closed,
    Draft,
    Conflict,
    /// A required check or review is missing.
    Blocked,
    /// The base branch moved and the repository requires an up-to-date branch.
    Behind,
    /// UNSTABLE, without `allowFailingChecks`, or in bulk.
    ChecksFailing,
    /// GitHub has not worked out whether it can merge.
    NotReady,
    HeadMoved,
    /// GitHub gave no merge method: the App cannot see the repository's settings.
    NoMethods,
    MethodRefused,
    ForkInBulk,
    ForkUnconfirmed,
    ForkByAgent,
}

/// The decision for `pull`, read just now, in a repository that allows `methods`.
#[must_use]
pub fn decide(pull: &PullDetail, methods: &[MergeMethod], asked: &Ask<'_>) -> Decision {
    if pull.pull.state == PullState::Merged {
        return Decision::AlreadyMerged;
    }
    refusal(pull, methods, asked).map_or(Decision::Merge, Decision::Refused)
}

/// The first rule `pull` breaks, in order: the fork rule, then its state, the head, the
/// method and its mergeability.
fn refusal(pull: &PullDetail, methods: &[MergeMethod], asked: &Ask<'_>) -> Option<Refusal> {
    let read = &pull.pull;
    if read.from_fork
        && let Some(refused) = fork(read.head_repository.as_deref(), asked)
    {
        return Some(refused);
    }
    if read.state != PullState::Open {
        return Some(Refusal::Closed);
    }
    if !read.head_sha.eq_ignore_ascii_case(asked.expected_head) {
        return Some(Refusal::HeadMoved);
    }
    if read.draft {
        return Some(Refusal::Draft);
    }
    if methods.is_empty() {
        return Some(Refusal::NoMethods);
    }
    if !methods.contains(&asked.method) {
        return Some(Refusal::MethodRefused);
    }
    mergeability(pull, asked.merging)
}

/// The fork rule (PRO-013): never in bulk, never for an agent, and only when the caller typed
/// the head repository exactly. A deleted head repository cannot be typed, so it never merges.
fn fork(head_repository: Option<&str>, asked: &Ask<'_>) -> Option<Refusal> {
    match asked.merging {
        Merging::InBulk => Some(Refusal::ForkInBulk),
        _ if asked.agent => Some(Refusal::ForkByAgent),
        Merging::Alone { fork_confirmed, .. } => match (fork_confirmed, head_repository) {
            (Some(typed), Some(head)) if typed == head => None,
            _ => Some(Refusal::ForkUnconfirmed),
        },
    }
}

/// Whether GitHub's mergeability lets it merge. Only CLEAN merges in bulk; UNSTABLE merges
/// alone with `allowFailingChecks`. BLOCKED never: the hub does not bypass a required check.
fn mergeability(pull: &PullDetail, merging: Merging<'_>) -> Option<Refusal> {
    let unstable_allowed = matches!(
        merging,
        Merging::Alone {
            allow_failing_checks: true,
            ..
        }
    );
    let refused = match merge_state(pull.mergeable_state) {
        MergeState::Clean => None,
        MergeState::Unstable if unstable_allowed => None,
        MergeState::Unstable => Some(Refusal::ChecksFailing),
        MergeState::Conflict => Some(Refusal::Conflict),
        MergeState::Blocked => Some(Refusal::Blocked),
        MergeState::Behind => Some(Refusal::Behind),
        MergeState::Draft => Some(Refusal::Draft),
        MergeState::Unknown => Some(Refusal::NotReady),
    };
    refused.or(match pull.mergeable {
        Some(true) => None,
        Some(false) => Some(Refusal::Conflict),
        None => Some(Refusal::NotReady),
    })
}

impl Refusal {
    /// The fixed words: never a reply of GitHub or text from the pull request (LOG-007).
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Closed => "It cannot merge now: it is closed.",
            Self::Draft => "It cannot merge now: it is a draft.",
            Self::Conflict => "It cannot merge now: it has a conflict.",
            Self::Blocked => "It cannot merge now: a required check or review is missing.",
            Self::Behind => "It cannot merge now: the base branch moved. Update the branch.",
            Self::ChecksFailing => {
                "It cannot merge now: a check failed or still runs. Merge it alone with \
                 allowFailingChecks."
            }
            Self::NotReady => "GitHub has not worked out whether it can merge. Try again.",
            Self::HeadMoved => "The pull request changed since you looked. Read it again.",
            Self::NoMethods => {
                "The hub cannot see which merge methods the repository allows. Give the GitHub \
                 App Contents write."
            }
            Self::MethodRefused => "The repository does not allow that merge method.",
            Self::ForkInBulk => {
                "It is from a fork: merge it alone, after you type its head repository (PRO-013)."
            }
            Self::ForkUnconfirmed => {
                "It is from a fork: set forkConfirmed to its head repository to merge it \
                 (PRO-013)."
            }
            Self::ForkByAgent => {
                "An AI agent cannot merge a pull request from a fork. The operator merges it \
                 (PRO-013)."
            }
        }
    }

    /// The K7 code of the refusal of one merge: the fork rule is `FORBIDDEN`, the rest
    /// `CONFLICT`.
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::ForkInBulk | Self::ForkUnconfirmed | Self::ForkByAgent => ErrorCode::Forbidden,
            _ => ErrorCode::Conflict,
        }
    }

    /// Its outcome in a bulk merge.
    #[must_use]
    pub const fn outcome(self) -> MergeOutcome {
        match self {
            Self::Closed
            | Self::Draft
            | Self::Conflict
            | Self::Blocked
            | Self::Behind
            | Self::ChecksFailing => MergeOutcome::NotMergeable,
            Self::NotReady => MergeOutcome::NotReady,
            Self::HeadMoved => MergeOutcome::HeadMoved,
            Self::NoMethods | Self::MethodRefused => MergeOutcome::MethodRefused,
            Self::ForkInBulk | Self::ForkUnconfirmed | Self::ForkByAgent => {
                MergeOutcome::ForkRefused
            }
        }
    }
}
