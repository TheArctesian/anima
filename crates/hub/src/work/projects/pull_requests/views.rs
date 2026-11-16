//! The K1 shapes of pull requests, from the provider's types. Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::pulls::{self as github, MergeableState, PullState};

use crate::{
    paging::PageEnd,
    work::projects::forge::{
        Asked, ForgeRead, int,
        text::{self, LABELS, TITLE},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum MergeState {
    /// No conflict and the checks passed.
    Clean,
    /// No conflict; a check that is not required failed or still runs.
    Unstable,
    Conflict,
    /// A required check or review is missing.
    Blocked,
    /// The base branch moved and the repository requires an up-to-date branch.
    Behind,
    Draft,
    /// GitHub has not computed it yet. Ask again.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "PullRequestMergeability")]
pub struct Mergeability {
    state: MergeState,
    /// True only for CLEAN on an open pull request that is not a draft (PRJ-033).
    can_merge: bool,
}

/// GitHub's `mergeable_state` as K1's state (decisions item 4). `has_hooks` is GitHub's name
/// for a clean pull request in a repository with pre-receive hooks: no conflict and the checks
/// passed, so it is CLEAN.
#[must_use]
pub const fn merge_state(state: MergeableState) -> MergeState {
    match state {
        MergeableState::Clean | MergeableState::HasHooks => MergeState::Clean,
        MergeableState::Unstable => MergeState::Unstable,
        MergeableState::Dirty => MergeState::Conflict,
        MergeableState::Blocked => MergeState::Blocked,
        MergeableState::Behind => MergeState::Behind,
        MergeableState::Draft => MergeState::Draft,
        MergeableState::Unknown => MergeState::Unknown,
    }
}

/// The mergeability of the single read `detail`; `None` for one that is not open. It can
/// merge only when it is CLEAN, open, not a draft, and GitHub says `mergeable`.
#[must_use]
pub fn mergeability(detail: &github::PullDetail) -> Option<Mergeability> {
    if detail.pull.state != PullState::Open {
        return None;
    }
    let state = merge_state(detail.mergeable_state);
    Some(Mergeability {
        state,
        can_merge: state == MergeState::Clean
            && !detail.pull.draft
            && detail.mergeable == Some(true),
    })
}

#[must_use]
pub const fn merge_method(method: github::MergeMethod) -> MergeMethod {
    match method {
        github::MergeMethod::Merge => MergeMethod::Merge,
        github::MergeMethod::Squash => MergeMethod::Squash,
        github::MergeMethod::Rebase => MergeMethod::Rebase,
    }
}

/// What the hub knows of a pull request's mergeability before its lazy field runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    /// A list entry: the single read has not been made.
    Unread,
    /// The single read gave it: `None` for one that is not open.
    Read(Option<Mergeability>),
}

/// Identity is the repository and the number. It has no hub ID: the hub does not store it.
#[derive(SimpleObject)]
#[graphql(name = "PullRequest", complex)]
pub struct PullRequestView {
    repository: ID,
    number: i32,
    /// At most 300 characters. Text from GitHub.
    title: String,
    pub state: PullRequestState,
    draft: bool,
    /// Text from GitHub.
    author_login: String,
    /// Text from GitHub.
    head_branch: String,
    /// 40 lower-case hex digits.
    pub head_sha: String,
    base_branch: String,
    /// True: the head branch is in another repository. Such a pull request is never merged in bulk, and its workflow runs are not run again from Anima (PRO-013).
    from_fork: bool,
    /// owner/name of the head repository; null when it was deleted. Text from GitHub.
    head_repository: Option<String>,
    /// Up to 20 names. Text from GitHub.
    labels: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    /// The page on GitHub (PRJ-010).
    url: String,
    /// Markdown source, cut at 20,000 characters. Text from GitHub: plain text, never HTML.
    body: Option<String>,
    body_cut: bool,
    /// The number GitHub uses, for the lazy fields.
    #[graphql(skip)]
    pub github_number: u64,
    /// The count of comments when the read gave it (a list entry does not).
    #[graphql(skip)]
    pub comment_count: Option<u64>,
    #[graphql(skip)]
    pub known_mergeability: Known,
    #[graphql(skip)]
    pub asked: Asked,
}

/// The K1 pull request of `pull`, read for `asked`.
#[must_use]
pub fn pull_request(pull: github::PullRequest, asked: Asked) -> PullRequestView {
    let (body, body_cut) = text::body(pull.body);
    PullRequestView {
        repository: ID(asked.target.repository.to_string()),
        number: int(pull.number),
        title: text::capped(pull.title, TITLE),
        state: match pull.state {
            PullState::Open => PullRequestState::Open,
            PullState::Closed => PullRequestState::Closed,
            PullState::Merged => PullRequestState::Merged,
        },
        draft: pull.draft,
        author_login: pull.author_login,
        head_branch: pull.head_branch,
        head_sha: pull.head_sha,
        base_branch: pull.base_branch,
        from_fork: pull.from_fork,
        head_repository: pull.head_repository,
        labels: text::first(pull.labels, LABELS),
        created_at: pull.created_at,
        updated_at: pull.updated_at,
        merged_at: pull.merged_at,
        url: pull.url,
        body,
        body_cut,
        github_number: pull.number,
        comment_count: pull.comments,
        known_mergeability: Known::Unread,
        asked,
    }
}

/// The K1 pull request of the single read `detail`, its mergeability known.
#[must_use]
pub fn detailed(detail: github::PullDetail, asked: Asked) -> PullRequestView {
    let known = mergeability(&detail);
    PullRequestView {
        known_mergeability: Known::Read(known),
        ..pull_request(detail.pull, asked)
    }
}

#[derive(SimpleObject)]
#[graphql(name = "PullRequestEdge")]
pub struct PullRequestEdge {
    pub cursor: String,
    pub node: PullRequestView,
}

#[derive(SimpleObject)]
#[graphql(name = "PullRequestConnection", complex)]
pub struct PullRequestConnection {
    pub edges: Vec<PullRequestEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
    #[graphql(skip)]
    pub asked: Asked,
}
