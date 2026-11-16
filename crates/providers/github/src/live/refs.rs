//! The branches and the releases of a repository (PRJ-033).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::{User, get, login, read};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub head_sha: String,
    pub protected: bool,
}

#[derive(Deserialize)]
struct BranchHead {
    sha: String,
}

#[derive(Deserialize)]
struct RawBranch {
    name: String,
    commit: BranchHead,
    protected: Option<bool>,
}

/// `GET /repos/{o}/{r}/branches`, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn branches_request(repository: &str, page: u32) -> Result<Request, Problem> {
    get(repository, "/branches", &[], Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of branches.
pub fn branches_parse(response: &Response<'_>) -> Result<Vec<Branch>, Problem> {
    let raw: Vec<RawBranch> = read(response, "branches")?;
    Ok(raw
        .into_iter()
        .map(|branch| Branch {
            name: branch.name,
            head_sha: branch.commit.sha,
            protected: branch.protected.unwrap_or(false),
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub id: u64,
    pub tag: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    /// `None` for a draft.
    pub published_at: Option<DateTime<Utc>>,
    pub author_login: String,
    pub url: String,
    pub body: String,
}

#[derive(Deserialize)]
struct RawRelease {
    id: u64,
    tag_name: String,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<DateTime<Utc>>,
    author: Option<User>,
    html_url: String,
    body: Option<String>,
}

/// `GET /repos/{o}/{r}/releases`, 100 to a page. A key with write access sees drafts; they
/// come back with `draft` true and the hub decides whether to show them.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn releases_request(repository: &str, page: u32) -> Result<Request, Problem> {
    get(repository, "/releases", &[], Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of releases.
pub fn releases_parse(response: &Response<'_>) -> Result<Vec<Release>, Problem> {
    let raw: Vec<RawRelease> = read(response, "releases")?;
    Ok(raw
        .into_iter()
        .map(|release| Release {
            id: release.id,
            tag: release.tag_name,
            name: release.name,
            draft: release.draft,
            prerelease: release.prerelease,
            published_at: release.published_at,
            author_login: login(release.author),
            url: release.html_url,
            body: release.body.unwrap_or_default(),
        })
        .collect())
}
