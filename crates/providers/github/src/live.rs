//! Live reads of one repository: pull requests, issues, comments, workflow runs and checks
//! (PRJ-030, PRJ-031, PRJ-032).
//!
//! Pure, like the sync: each module builds a [`Request`] and parses the [`Response`] to plain
//! types of this crate. The hub sends the request and reads the `Link` header itself, and
//! takes only the page number from it ([`paging::next_page`]). Text from GitHub (titles,
//! bodies, logins, names) stays a plain `String`; the hub cuts it and never renders it as
//! HTML. A value GitHub may add to an open set maps to an `Unknown` or `Other` variant, so
//! one new value never fails a whole list.

pub mod checks;
pub mod comments;
pub mod issues;
pub mod paging;
pub mod pulls;
pub mod runs;
pub mod workflows;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use source_exchange::{Body, Problem, Request, Response};

use crate::name;
use crate::problem::problem;

/// Which pull requests or issues a list asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateFilter {
    Open,
    Closed,
    All,
}

impl StateFilter {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

/// `GET /repos/{repository}{tail}` with `filters`, and with the page when one is given. The
/// repository is `owner/name`; one that cannot go into a path is refused before any path is
/// built, with fixed words.
pub(crate) fn get(
    repository: &str,
    tail: &str,
    filters: &[(&str, &str)],
    page: Option<u32>,
) -> Result<Request, Problem> {
    name::check(repository).map_err(|name::UnusableName| {
        problem(
            "repository_name_unusable",
            "repository",
            "the repository name cannot be used to read it",
        )
    })?;
    let path = format!("/repos/{repository}{tail}");
    let result = match page {
        Some(page) => {
            let page = page.max(1).to_string();
            let mut all = filters.to_vec();
            all.extend(paging::query(&page));
            Request::get(&path, &all)
        }
        None => Request::get(&path, filters),
    };
    result.map_err(|_| {
        problem(
            "request_unusable",
            "repository",
            "the request could not be built",
        )
    })
}

/// The body of a 200 reply as `T`. Any other status, a body larger than the hub reads and a
/// body that is not `T` are each a problem with fixed words, never a piece of GitHub's reply.
pub(crate) fn read<T: DeserializeOwned>(
    response: &Response<'_>,
    subject: &str,
) -> Result<T, Problem> {
    if response.status != 200 {
        return Err(problem(
            "unexpected_status",
            subject,
            &format!("GitHub answered with status {}", response.status),
        ));
    }
    let Body::Read(body) = response.body else {
        return Err(problem(
            "reply_too_large",
            subject,
            "GitHub's reply is larger than the hub reads",
        ));
    };
    serde_json::from_slice(body).map_err(|_| unreadable(subject))
}

pub(crate) fn unreadable(subject: &str) -> Problem {
    problem(
        "reply_unreadable",
        subject,
        "GitHub's reply could not be read",
    )
}

/// The login GitHub shows for an account that no longer exists.
const GHOST: &str = "ghost";

#[derive(Deserialize)]
pub(crate) struct User {
    login: String,
}

/// A login, or `ghost` when GitHub gives no user (a deleted account).
pub(crate) fn login(user: Option<User>) -> String {
    user.map_or_else(|| GHOST.to_owned(), |user| user.login)
}

/// A label as GitHub sends it: an object with a name (a bare name is read too).
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Label {
    Name(String),
    Object { name: Option<String> },
}

pub(crate) fn label_names(labels: Vec<Label>) -> Vec<String> {
    labels
        .into_iter()
        .filter_map(|label| match label {
            Label::Name(name) | Label::Object { name: Some(name) } => Some(name),
            Label::Object { name: None } => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
