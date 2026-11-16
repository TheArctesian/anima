//! The K1 shapes of issues and their comments, from the provider's types. Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::{
    comments::Comment,
    issues::{CloseReason, Issue},
};
use serde::{Deserialize, de::IgnoredAny};

use crate::{
    ids::RepositoryId,
    paging::PageEnd,
    work::projects::forge::{
        ForgeRead, int,
        text::{self, LABELS, TITLE},
    },
};

/// The most assignees shown.
const ASSIGNEES: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum IssueCloseReason {
    Completed,
    NotPlanned,
}

#[derive(SimpleObject)]
#[graphql(name = "Issue")]
pub struct IssueView {
    repository: ID,
    number: i32,
    /// At most 300 characters. Text from GitHub.
    title: String,
    open: bool,
    /// Set for a closed issue when GitHub gives it.
    close_reason: Option<IssueCloseReason>,
    /// Text from GitHub.
    author_login: String,
    /// Up to 20 names. Text from GitHub.
    labels: Vec<String>,
    /// Up to 10 logins. Text from GitHub.
    assignees: Vec<String>,
    comments: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    url: String,
    /// Markdown source, cut at 20,000 characters. Text from GitHub: plain text, never HTML.
    body: Option<String>,
    body_cut: bool,
}

/// GitHub's close reason as K1's: only for a closed issue, and only the two K1 names
/// (`reopened`, `duplicate` and any other value are not shown).
const fn close_reason(open: bool, reason: Option<CloseReason>) -> Option<IssueCloseReason> {
    match (open, reason) {
        (false, Some(CloseReason::Completed)) => Some(IssueCloseReason::Completed),
        (false, Some(CloseReason::NotPlanned)) => Some(IssueCloseReason::NotPlanned),
        _ => None,
    }
}

/// The K1 issue of `issue` in `repository`.
#[must_use]
pub fn issue(issue: Issue, repository: RepositoryId) -> IssueView {
    let (body, body_cut) = text::body(issue.body);
    IssueView {
        repository: ID(repository.to_string()),
        number: int(issue.number),
        title: text::capped(issue.title, TITLE),
        open: issue.open,
        close_reason: close_reason(issue.open, issue.close_reason),
        author_login: issue.author_login,
        labels: text::first(issue.labels, LABELS),
        assignees: text::first(issue.assignees, ASSIGNEES),
        comments: int(issue.comments),
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        closed_at: issue.closed_at,
        url: issue.url,
        body,
        body_cut,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "IssueComment")]
pub struct CommentView {
    /// GitHub's comment number, as digits.
    id: ID,
    /// Text from GitHub.
    author_login: String,
    /// Markdown source, cut at 20,000 characters. Text from GitHub: plain text, never HTML.
    body: String,
    body_cut: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    url: String,
}

#[must_use]
pub fn comment(comment: Comment) -> CommentView {
    let (body, body_cut) = text::cut(comment.body, text::BODY);
    CommentView {
        id: ID(comment.id.to_string()),
        author_login: comment.author_login,
        body,
        body_cut,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
        url: comment.url,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "IssueEdge")]
pub struct IssueEdge {
    pub cursor: String,
    pub node: IssueView,
}

#[derive(SimpleObject)]
#[graphql(name = "IssueConnection")]
pub struct IssueConnection {
    pub edges: Vec<IssueEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}

#[derive(SimpleObject)]
#[graphql(name = "IssueCommentEdge")]
pub struct CommentEdge {
    pub cursor: String,
    pub node: CommentView,
}

#[derive(SimpleObject)]
#[graphql(name = "IssueCommentConnection")]
pub struct CommentConnection {
    pub edges: Vec<CommentEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}

/// The entries of one page of GitHub's issue list, pull requests included: whether the page
/// was full is told by this count, not by the issues the provider keeps.
#[must_use]
pub fn entries(body: &[u8]) -> usize {
    serde_json::from_slice::<Vec<IgnoredAny>>(body).map_or(0, |entries| entries.len())
}

#[derive(Deserialize)]
struct Marked {
    pull_request: Option<IgnoredAny>,
}

/// GitHub's issue `body` is a pull request: it has the `pull_request` key.
#[must_use]
pub fn is_pull_request(body: &[u8]) -> bool {
    serde_json::from_slice::<Marked>(body).is_ok_and(|marked| marked.pull_request.is_some())
}
