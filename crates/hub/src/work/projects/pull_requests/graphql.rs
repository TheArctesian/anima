//! The pull request roots (K1): a page of a repository's pull requests and one pull request,
//! with the fields that cost a GitHub request each only when they are selected.

use async_graphql::{ComplexObject, Context, ID, Object, Result};
use provider_github::live::pulls::{self as github, PullDetail};

use super::views::{
    Known, MergeMethod, Mergeability, PullRequestConnection, PullRequestEdge, PullRequestState,
    PullRequestView, detailed, merge_method, mergeability, pull_request,
};
use crate::{
    engine::sources::Keep,
    fleet::access::Authenticated,
    work::projects::forge::{
        self, Asked, CommitChecksView, GITHUB_PAGE, OpenOrClosed, cut, int, missing,
    },
};

/// K1's default for `first`, and its most.
const DEFAULT_FIRST: i32 = 20;
const MOST: usize = 50;
/// How long each answer stays fresh (K-E1 Section 13.1).
const LIST: Keep = Keep::for_seconds(30);
pub(super) const ONE: Keep = Keep::for_seconds(15);
const SETTINGS: Keep = Keep::for_seconds(600);

/// The single read of `view`'s pull request, which holds its mergeability and comment count.
async fn detail(ctx: &Context<'_>, view: &PullRequestView) -> Result<Option<PullDetail>> {
    let asked = &view.asked;
    let request = github::one_request(&asked.target.name, view.github_number);
    match forge::read(ctx, &asked.target, request, ONE.fresh(asked.fresh)).await? {
        Some(reply) => forge::parsed(&reply, github::one_parse).map(Some),
        None => Ok(None),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "PullRequestsQuery")]
impl Query {
    /// Newest updated first. first is 1 to 50. One GitHub request, and more for mergeability and checks when selected (PRJ-031, PRJ-033).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.pullRequests"
    )]
    async fn pull_requests(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(default_with = "Some(OpenOrClosed::Open)")] state: Option<OpenOrClosed>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<PullRequestConnection> {
        let target = forge::target(ctx, &repository).await?;
        let (size, place) = forge::paging(first.unwrap_or(DEFAULT_FIRST), after.as_deref(), MOST)?;
        let asked = Asked {
            target,
            fresh: fresh.unwrap_or(false),
        };
        let state = state.unwrap_or(OpenOrClosed::Open).filter();
        let request = github::list_request(&asked.target.name, state, place.page);
        let reply = forge::read(ctx, &asked.target, request, LIST.fresh(asked.fresh))
            .await?
            .ok_or_else(|| missing("repository"))?;
        let pulls = forge::parsed(&reply, github::list_parse)?;
        let cut = cut(place, size, pulls.len(), pulls.len() >= GITHUB_PAGE);
        let (edges, page_info) = forge::page(pulls, cut, |(cursor, pull)| PullRequestEdge {
            cursor,
            node: pull_request(pull, asked.clone()),
        });
        Ok(PullRequestConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
            asked,
        })
    }

    /// Null when the repository has no pull request with this number.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.pullRequest"
    )]
    async fn pull_request(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        number: i32,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<Option<PullRequestView>> {
        let target = forge::target(ctx, &repository).await?;
        let number = forge::number(number)?;
        let asked = Asked {
            target,
            fresh: fresh.unwrap_or(false),
        };
        let request = github::one_request(&asked.target.name, number);
        let Some(reply) = forge::read(ctx, &asked.target, request, ONE.fresh(asked.fresh)).await?
        else {
            return Ok(None);
        };
        let detail = forge::parsed(&reply, github::one_parse)?;
        Ok(Some(detailed(detail, asked)))
    }
}

#[ComplexObject]
impl PullRequestView {
    // A list entry has no count of comments: the single read gives it.
    async fn comments(&self, ctx: &Context<'_>) -> Result<i32> {
        if let Some(count) = self.comment_count {
            return Ok(int(count));
        }
        let detail = detail(ctx, self)
            .await?
            .ok_or_else(|| missing("pull request"))?;
        Ok(detail.pull.comments.map_or(0, int))
    }

    /// Costs one GitHub request for each pull request when selected. Null for one that is not open.
    async fn mergeability(&self, ctx: &Context<'_>) -> Result<Option<Mergeability>> {
        if let Known::Read(known) = self.known_mergeability {
            return Ok(known);
        }
        if self.state != PullRequestState::Open {
            return Ok(None);
        }
        Ok(detail(ctx, self).await?.and_then(|d| mergeability(&d)))
    }

    /// The checks of headSha. Costs two GitHub requests for each pull request when selected.
    async fn checks(&self, ctx: &Context<'_>) -> Result<Option<CommitChecksView>> {
        let asked = &self.asked;
        forge::commit_checks(ctx, &asked.target, &self.head_sha, asked.fresh).await
    }
}

#[ComplexObject]
impl PullRequestConnection {
    /// The merge methods the repository allows. Costs one GitHub request when selected (kept 10 minutes).
    async fn merge_methods(&self, ctx: &Context<'_>) -> Result<Vec<MergeMethod>> {
        let asked = &self.asked;
        let request = github::merge_methods_request(&asked.target.name);
        let reply = forge::read(ctx, &asked.target, request, SETTINGS.fresh(asked.fresh))
            .await?
            .ok_or_else(|| missing("repository"))?;
        let methods = forge::parsed(&reply, github::merge_methods_parse)?;
        Ok(methods.into_iter().map(merge_method).collect())
    }
}
