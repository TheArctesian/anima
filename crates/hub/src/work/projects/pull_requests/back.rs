//! A pull request read from GitHub around a merge: the pull request and the merge methods just
//! before it, never from the cache and never stale, and the pull request read back once GitHub
//! merged it (K1 returns it; K-E1 Section 13.3).

use async_graphql::{Context, Result};
use provider_github::live::pulls::{self as github, PullDetail};

use super::{
    graphql::ONE,
    views::{MergeMethod, PullRequestView, detailed, merge_method},
};
use crate::{
    engine::sources::Keep,
    work::projects::forge::{self, Asked, Target, failure, missing},
};

/// Pull request `number` as GitHub gives it now; `None` when GitHub has no such pull request.
///
/// # Errors
/// The fixed words of a failed read.
pub(super) async fn before(
    ctx: &Context<'_>,
    target: &Target,
    number: u64,
) -> Result<Option<PullDetail>> {
    let request = github::one_request(&target.name, number);
    match forge::read(ctx, target, request, Keep::never()).await? {
        Some(reply) => forge::parsed(&reply, github::one_parse).map(Some),
        None => Ok(None),
    }
}

/// The merge methods the repository allows now: none when GitHub leaves them out (decisions
/// item 3).
///
/// # Errors
/// `NOT_FOUND` when GitHub has no such repository; the fixed words of a failed read.
pub(super) async fn methods(ctx: &Context<'_>, target: &Target) -> Result<Vec<MergeMethod>> {
    let request = github::merge_methods_request(&target.name);
    let reply = forge::read(ctx, target, request, Keep::never())
        .await?
        .ok_or_else(|| missing("repository"))?;
    let methods = forge::parsed(&reply, github::merge_methods_parse)?;
    Ok(methods.into_iter().map(merge_method).collect())
}

/// Pull request `number` after GitHub merged it.
///
/// # Errors
/// `UNAVAILABLE` with [`failure::unread`] when it cannot be read: the merge stands.
pub(super) async fn merged(
    ctx: &Context<'_>,
    target: &Target,
    number: u64,
) -> Result<PullRequestView> {
    let request = github::one_request(&target.name, number);
    let reply = forge::read(ctx, target, request, ONE.fresh(true))
        .await
        .ok()
        .flatten()
        .ok_or_else(failure::unread)?;
    let detail = forge::parsed(&reply, github::one_parse).map_err(|_| failure::unread())?;
    let asked = Asked {
        target: target.clone(),
        fresh: false,
    };
    Ok(detailed(detail, asked))
}
