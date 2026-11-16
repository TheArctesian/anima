//! The K1 shapes of commits, branches and releases, from the provider's types. Pure.

use async_graphql::{ID, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::{commits::Commit, refs::Branch, refs::Release};

use crate::{
    paging::PageEnd,
    work::projects::forge::{Asked, ForgeRead, text},
};

/// The most characters of a commit's title, and of the rest of its message.
const COMMIT_TITLE: usize = 200;
const COMMIT_MESSAGE: usize = 5_000;

#[derive(SimpleObject)]
#[graphql(name = "Commit", complex)]
pub struct CommitView {
    repository: ID,
    /// 40 lower-case hex digits.
    pub sha: String,
    /// The first line of the message, at most 200 characters. Text from GitHub.
    title: String,
    /// The rest of the message, cut at 5,000 characters. Text from GitHub.
    message: Option<String>,
    /// The name in the commit. Text from GitHub.
    author_name: String,
    /// The GitHub login, when GitHub matched one. Text from GitHub.
    author_login: Option<String>,
    committed_at: DateTime<Utc>,
    parents: Vec<String>,
    url: String,
    #[graphql(skip)]
    pub asked: Asked,
}

/// The message after its first line, without the blank lines that part it from the title;
/// `None` when nothing follows.
fn rest(message: &str) -> Option<String> {
    let (_, rest) = message.split_once('\n')?;
    let rest = rest.trim_start_matches(['\r', '\n']);
    (!rest.is_empty()).then(|| text::capped(rest.to_owned(), COMMIT_MESSAGE))
}

/// The K1 commit of `commit`, with the shas of its `parents`, read for `asked`.
#[must_use]
pub fn commit(commit: Commit, parents: Vec<String>, asked: Asked) -> CommitView {
    CommitView {
        repository: ID(asked.target.repository.to_string()),
        message: rest(&commit.message),
        sha: commit.sha,
        title: text::capped(commit.title, COMMIT_TITLE),
        author_name: commit.author_name,
        author_login: commit.author_login,
        committed_at: commit.committed_at,
        parents,
        url: commit.url,
        asked,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "CommitEdge")]
pub struct CommitEdge {
    pub cursor: String,
    pub node: CommitView,
}

#[derive(SimpleObject)]
#[graphql(name = "CommitConnection")]
pub struct CommitConnection {
    pub edges: Vec<CommitEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}

#[derive(SimpleObject)]
#[graphql(name = "Branch")]
pub struct BranchView {
    /// Text from GitHub.
    name: String,
    sha: String,
    protected: bool,
    /// True for the repository's default branch.
    default: bool,
}

/// The K1 branch of `branch`; `default_branch` is the repository's, as the last sync read it.
#[must_use]
pub fn branch(branch: Branch, default_branch: Option<&str>) -> BranchView {
    BranchView {
        default: default_branch == Some(branch.name.as_str()),
        name: branch.name,
        sha: branch.head_sha,
        protected: branch.protected,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "BranchEdge")]
pub struct BranchEdge {
    pub cursor: String,
    pub node: BranchView,
}

#[derive(SimpleObject)]
#[graphql(name = "BranchConnection")]
pub struct BranchConnection {
    pub edges: Vec<BranchEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}

/// A release on GitHub. Not a release of Anima (type Release).
#[derive(SimpleObject)]
#[graphql(name = "RepositoryRelease")]
pub struct ReleaseView {
    /// GitHub's release number, as digits.
    id: ID,
    /// Text from GitHub.
    tag: String,
    /// Text from GitHub.
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    /// Text from GitHub.
    author_login: Option<String>,
    created_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
    url: String,
}

/// The K1 release of `release`, made at `created_at`.
#[must_use]
pub fn release(release: Release, created_at: DateTime<Utc>) -> ReleaseView {
    ReleaseView {
        id: ID(release.id.to_string()),
        tag: release.tag,
        name: release.name,
        draft: release.draft,
        prerelease: release.prerelease,
        author_login: Some(release.author_login),
        created_at,
        published_at: release.published_at,
        url: release.url,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "RepositoryReleaseEdge")]
pub struct ReleaseEdge {
    pub cursor: String,
    pub node: ReleaseView,
}

#[derive(SimpleObject)]
#[graphql(name = "RepositoryReleaseConnection")]
pub struct ReleaseConnection {
    pub edges: Vec<ReleaseEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}
