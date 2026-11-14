//! Pull requests of a repository, one pull request with its mergeability, and the merge
//! methods the repository allows (PRJ-030, PRJ-033).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::{Label, StateFilter, User, get, label_names, login, read};

const SUBJECT: &str = "pull requests";

/// Where a pull request stands. `Merged` is a closed pull request with a merge time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullState {
    Open,
    Closed,
    Merged,
}

/// GitHub's `mergeable_state`, an open string. `Unknown` is anything not named here, and
/// null: it is never read as mergeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeableState {
    Clean,
    Unstable,
    Dirty,
    Blocked,
    Behind,
    Draft,
    HasHooks,
    Unknown,
}

impl MergeableState {
    fn named(name: Option<&str>) -> Self {
        match name.map(str::to_ascii_lowercase).as_deref() {
            Some("clean") => Self::Clean,
            Some("unstable") => Self::Unstable,
            Some("dirty") => Self::Dirty,
            Some("blocked") => Self::Blocked,
            Some("behind") => Self::Behind,
            Some("draft") => Self::Draft,
            Some("has_hooks") => Self::HasHooks,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: PullState,
    pub draft: bool,
    pub author_login: String,
    pub head_branch: String,
    pub head_sha: String,
    pub base_branch: String,
    /// The head branch is in another repository, or that repository is gone.
    pub from_fork: bool,
    /// `owner/name` of the head repository; `None` when it was deleted.
    pub head_repository: Option<String>,
    pub labels: Vec<String>,
    /// Only the single read has it; a list entry gives `None`.
    pub comments: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub url: String,
    pub body: Option<String>,
}

/// One pull request as the single read gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullDetail {
    pub pull: PullRequest,
    /// `None` while GitHub computes it: read again later.
    pub mergeable: Option<bool>,
    pub mergeable_state: MergeableState,
}

#[derive(Deserialize)]
struct Repo {
    id: u64,
    full_name: String,
}

#[derive(Deserialize)]
struct Side {
    #[serde(rename = "ref")]
    branch: String,
    sha: String,
    repo: Option<Repo>,
}

#[derive(Deserialize)]
struct Raw {
    number: u64,
    title: String,
    state: String,
    draft: Option<bool>,
    user: Option<User>,
    head: Side,
    base: Side,
    #[serde(default)]
    labels: Vec<Label>,
    comments: Option<u64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    html_url: String,
    body: Option<String>,
    merged: Option<bool>,
    mergeable: Option<bool>,
    mergeable_state: Option<String>,
}

impl Raw {
    fn into_detail(self) -> PullDetail {
        let merged = self.merged_at.is_some() || self.merged == Some(true);
        let state = match (self.state.as_str(), merged) {
            ("open", _) => PullState::Open,
            (_, true) => PullState::Merged,
            _ => PullState::Closed,
        };
        let from_fork = !matches!(
            (&self.head.repo, &self.base.repo),
            (Some(head), Some(base)) if head.id == base.id
        );
        PullDetail {
            mergeable: self.mergeable,
            mergeable_state: MergeableState::named(self.mergeable_state.as_deref()),
            pull: PullRequest {
                number: self.number,
                title: self.title,
                state,
                draft: self.draft.unwrap_or(false),
                author_login: login(self.user),
                head_branch: self.head.branch,
                head_sha: self.head.sha,
                base_branch: self.base.branch,
                from_fork,
                head_repository: self.head.repo.map(|repo| repo.full_name),
                labels: label_names(self.labels),
                comments: self.comments,
                created_at: self.created_at,
                updated_at: self.updated_at,
                merged_at: self.merged_at,
                url: self.html_url,
                body: self.body,
            },
        }
    }
}

/// `GET /repos/{o}/{r}/pulls`, newest first, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn list_request(repository: &str, state: StateFilter, page: u32) -> Result<Request, Problem> {
    get(
        repository,
        "/pulls",
        &[
            ("state", state.as_str()),
            ("sort", "updated"),
            ("direction", "desc"),
        ],
        Some(page),
    )
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of pull requests.
pub fn list_parse(response: &Response<'_>) -> Result<Vec<PullRequest>, Problem> {
    let raw: Vec<Raw> = read(response, SUBJECT)?;
    Ok(raw.into_iter().map(|raw| raw.into_detail().pull).collect())
}

/// `GET /repos/{o}/{r}/pulls/{number}`: the read that holds `mergeable`.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn one_request(repository: &str, number: u64) -> Result<Request, Problem> {
    get(repository, &format!("/pulls/{number}"), &[], None)
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a pull request.
pub fn one_parse(response: &Response<'_>) -> Result<PullDetail, Problem> {
    read::<Raw>(response, SUBJECT).map(Raw::into_detail)
}

/// `GET /repos/{o}/{r}`: the repository itself, for the merge methods it allows.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn merge_methods_request(repository: &str) -> Result<Request, Problem> {
    get(repository, "", &[], None)
}

#[derive(Deserialize)]
struct Settings {
    /// Required, so a body that is not a repository is refused; never read.
    #[serde(rename = "id")]
    _id: u64,
    allow_merge_commit: Option<bool>,
    allow_squash_merge: Option<bool>,
    allow_rebase_merge: Option<bool>,
}

/// The methods the repository allows. A field GitHub leaves out (the App lacks Contents
/// write) counts as not allowed, so no method is guessed.
///
/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a repository.
pub fn merge_methods_parse(response: &Response<'_>) -> Result<Vec<MergeMethod>, Problem> {
    let settings: Settings = read(response, "repository")?;
    Ok([
        (settings.allow_merge_commit, MergeMethod::Merge),
        (settings.allow_squash_merge, MergeMethod::Squash),
        (settings.allow_rebase_merge, MergeMethod::Rebase),
    ]
    .into_iter()
    .filter_map(|(allowed, method)| (allowed == Some(true)).then_some(method))
    .collect())
}
