//! An issue or a comment read from GitHub around a change: the issue before it is closed or
//! opened, and what GitHub made, read back once the change was made (K1 returns it). The read
//! before never comes from the cache, fresh or stale; the read back is fresh: the change
//! dropped the held answers.

use async_graphql::{Context, Result};
use provider_github::live::{
    comments,
    issues::{self as github, Issue},
};
use source_exchange::Body;

use super::{
    graphql::{LIST, ONE},
    views::{self, CommentView, IssueView, is_pull_request},
};
use crate::{
    api_errors::{FieldError, invalid},
    engine::sources::Keep,
    work::projects::forge::{self, GITHUB_PAGE, Target, failure, missing},
};

/// Issue `number` as it is now, before a change to it.
///
/// # Errors
/// `NOT_FOUND` when GitHub has no such issue; `VALIDATION` when the number is a pull request;
/// the fixed words of a failed read.
pub(super) async fn before(ctx: &Context<'_>, target: &Target, number: u64) -> Result<Issue> {
    let request = github::one_request(&target.name, number);
    // Never a held answer, not even a stale one: the change acts on it (K-E1 Section 13.3).
    let reply = forge::read(ctx, target, request, Keep::never())
        .await?
        .ok_or_else(|| missing("issue"))?;
    if matches!(reply.response().body, Body::Read(body) if is_pull_request(body)) {
        return Err(invalid(&[FieldError::new(
            "number",
            "This number is a pull request, not an issue.",
        )]));
    }
    forge::parsed(&reply, github::one_parse)
}

/// Issue `number` after GitHub made a change to it.
///
/// # Errors
/// `UNAVAILABLE` with [`failure::unread`] when it cannot be read: the change stands.
pub(super) async fn issue(ctx: &Context<'_>, target: &Target, number: u64) -> Result<IssueView> {
    let found = one(ctx, target, number).await.ok_or_else(failure::unread)?;
    Ok(views::issue(found, target.repository))
}

/// The comment `id` GitHub just made on issue or pull request `number`: on the last page of
/// its comments, or the one before when more came meanwhile.
///
/// # Errors
/// `UNAVAILABLE` with [`failure::unread`] when it cannot be read: the comment stands.
pub(super) async fn comment(
    ctx: &Context<'_>,
    target: &Target,
    number: u64,
    id: u64,
) -> Result<CommentView> {
    let count = one(ctx, target, number)
        .await
        .ok_or_else(failure::unread)?
        .comments;
    for page in pages(count) {
        let request = comments::request(&target.name, number, page);
        let Ok(Some(reply)) = forge::read(ctx, target, request, LIST.fresh(true)).await else {
            break;
        };
        let Ok(found) = forge::parsed(&reply, comments::parse) else {
            break;
        };
        if let Some(made) = found.into_iter().find(|comment| comment.id == id) {
            return Ok(views::comment(made));
        }
    }
    Err(failure::unread())
}

/// Issue or pull request `number` as GitHub's issue path gives it; `None` when it cannot be
/// read.
async fn one(ctx: &Context<'_>, target: &Target, number: u64) -> Option<Issue> {
    let request = github::one_request(&target.name, number);
    let reply = forge::read(ctx, target, request, ONE.fresh(true))
        .await
        .ok()??;
    forge::parsed(&reply, github::one_parse).ok()
}

/// The pages a new comment can be on, when `count` comments are there: the last, then the one
/// before. Pure.
#[must_use]
pub fn pages(count: u64) -> Vec<u32> {
    let size = u64::try_from(GITHUB_PAGE).unwrap_or(u64::MAX);
    let last = u32::try_from(count.div_ceil(size).max(1)).unwrap_or(u32::MAX);
    std::iter::once(last)
        .chain((last > 1).then(|| last - 1))
        .collect()
}
