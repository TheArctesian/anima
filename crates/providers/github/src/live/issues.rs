//! Issues of a repository (PRJ-031). GitHub lists pull requests as issues; they are dropped.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use source_exchange::{Problem, Request, Response};

use super::{Label, StateFilter, User, get, label_names, login, read, unreadable};

const SUBJECT: &str = "issues";

/// GitHub's `state_reason`, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Completed,
    NotPlanned,
    Reopened,
    Duplicate,
    Other,
}

impl CloseReason {
    fn named(name: &str) -> Self {
        match name {
            "completed" => Self::Completed,
            "not_planned" => Self::NotPlanned,
            "reopened" => Self::Reopened,
            "duplicate" => Self::Duplicate,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub open: bool,
    pub close_reason: Option<CloseReason>,
    pub author_login: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub comments: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub url: String,
    pub body: Option<String>,
}

#[derive(Deserialize)]
struct Raw {
    number: u64,
    title: String,
    state: String,
    state_reason: Option<String>,
    user: Option<User>,
    #[serde(default)]
    labels: Vec<Label>,
    assignees: Option<Vec<User>>,
    #[serde(default)]
    comments: u64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    html_url: String,
    body: Option<String>,
}

impl Raw {
    fn into_issue(self) -> Issue {
        Issue {
            number: self.number,
            title: self.title,
            open: self.state == "open",
            close_reason: self.state_reason.as_deref().map(CloseReason::named),
            author_login: login(self.user),
            labels: label_names(self.labels),
            assignees: self
                .assignees
                .unwrap_or_default()
                .into_iter()
                .map(|user| user.login)
                .collect(),
            comments: self.comments,
            created_at: self.created_at,
            updated_at: self.updated_at,
            closed_at: self.closed_at,
            url: self.html_url,
            body: self.body,
        }
    }
}

/// `GET /repos/{o}/{r}/issues`, newest first, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn list_request(repository: &str, state: StateFilter, page: u32) -> Result<Request, Problem> {
    get(
        repository,
        "/issues",
        &[
            ("state", state.as_str()),
            ("sort", "created"),
            ("direction", "desc"),
        ],
        Some(page),
    )
}

/// The issues of one page. An entry with the `pull_request` key is a pull request and is
/// dropped, so a page can hold fewer than 100.
///
/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of issues.
pub fn list_parse(response: &Response<'_>) -> Result<Vec<Issue>, Problem> {
    let entries: Vec<Value> = read(response, SUBJECT)?;
    entries
        .into_iter()
        .filter(|entry| entry.get("pull_request").is_none())
        .map(|entry| {
            Raw::deserialize(entry)
                .map(Raw::into_issue)
                .map_err(|_| unreadable(SUBJECT))
        })
        .collect()
}

/// `GET /repos/{o}/{r}/issues/{number}`.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn one_request(repository: &str, number: u64) -> Result<Request, Problem> {
    get(repository, &format!("/issues/{number}"), &[], None)
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not an issue.
pub fn one_parse(response: &Response<'_>) -> Result<Issue, Problem> {
    read::<Raw>(response, SUBJECT).map(Raw::into_issue)
}
