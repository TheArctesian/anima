//! The comments of an issue or a pull request, oldest first (PRJ-031).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::{User, get, login, read};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub id: u64,
    pub author_login: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub url: String,
}

#[derive(Deserialize)]
struct Raw {
    id: u64,
    user: Option<User>,
    body: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    html_url: String,
}

/// `GET /repos/{o}/{r}/issues/{number}/comments`: GitHub numbers a pull request's
/// conversation the same way as an issue's.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn request(repository: &str, number: u64, page: u32) -> Result<Request, Problem> {
    get(
        repository,
        &format!("/issues/{number}/comments"),
        &[],
        Some(page),
    )
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of comments.
pub fn parse(response: &Response<'_>) -> Result<Vec<Comment>, Problem> {
    let raw: Vec<Raw> = read(response, "comments")?;
    Ok(raw
        .into_iter()
        .map(|raw| Comment {
            id: raw.id,
            author_login: login(raw.user),
            body: raw.body.unwrap_or_default(),
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            url: raw.html_url,
        })
        .collect())
}
