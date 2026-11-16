//! The commits of a branch or a sha, newest first (PRJ-033).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::path::ref_value;
use super::{get, read};

const SUBJECT: &str = "commits";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    /// The first line of the message, whole.
    pub title: String,
    /// The whole message, title included.
    pub message: String,
    pub author_name: String,
    /// The GitHub account GitHub matched to the author, when it matched one.
    pub author_login: Option<String>,
    pub authored_at: DateTime<Utc>,
    /// The committer's date; the authored date when GitHub gives no committer.
    pub committed_at: DateTime<Utc>,
    pub url: String,
    pub parents: usize,
}

#[derive(Deserialize)]
struct Person {
    name: Option<String>,
    date: DateTime<Utc>,
}

#[derive(Deserialize)]
struct Detail {
    message: String,
    author: Person,
    committer: Option<Person>,
}

/// GitHub's account for a commit: absent when no account matches, sometimes an empty object.
#[derive(Deserialize)]
struct Account {
    login: Option<String>,
}

#[derive(Deserialize)]
struct Parent {}

#[derive(Deserialize)]
struct Raw {
    sha: String,
    html_url: String,
    commit: Detail,
    author: Option<Account>,
    parents: Vec<Parent>,
}

impl Raw {
    fn into_commit(self) -> Commit {
        let authored_at = self.commit.author.date;
        Commit {
            sha: self.sha,
            title: self
                .commit
                .message
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned(),
            message: self.commit.message,
            author_name: self.commit.author.name.unwrap_or_default(),
            author_login: self.author.and_then(|account| account.login),
            authored_at,
            committed_at: self.commit.committer.map_or(authored_at, |c| c.date),
            url: self.html_url,
            parents: self.parents.len(),
        }
    }
}

/// `GET /repos/{o}/{r}/commits`, 100 to a page, starting from `git_ref` (a branch name or a
/// commit sha; the default branch when `None`). The ref goes in as the query value `sha`.
///
/// # Errors
/// A [`Problem`] when the repository name or the ref cannot go into a request.
pub fn list_request(
    repository: &str,
    git_ref: Option<&str>,
    page: u32,
) -> Result<Request, Problem> {
    let mut filters = Vec::new();
    if let Some(git_ref) = git_ref {
        filters.push(("sha", ref_value(git_ref, SUBJECT)?));
    }
    get(repository, "/commits", &filters, Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of commits.
pub fn list_parse(response: &Response<'_>) -> Result<Vec<Commit>, Problem> {
    let raw: Vec<Raw> = read(response, SUBJECT)?;
    Ok(raw.into_iter().map(Raw::into_commit).collect())
}
