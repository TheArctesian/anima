//! The issue roots (K1): a page of a repository's issues, one issue, and a page of the
//! comments of an issue or a pull request.

use async_graphql::{Context, ID, Object, Result};
use provider_github::live::{comments, issues as github};
use source_exchange::Body;

use super::views::{
    CommentConnection, CommentEdge, IssueConnection, IssueEdge, IssueView, comment, entries,
    is_pull_request, issue,
};
use crate::{
    engine::sources::Keep,
    fleet::access::Authenticated,
    work::projects::forge::{self, GITHUB_PAGE, OpenOrClosed, cut, missing},
};

/// K1's defaults for `first`, and their most.
const DEFAULT_FIRST: i32 = 30;
const MOST: usize = 100;
/// How long each answer stays fresh (K-E1 Section 13.1).
pub(super) const LIST: Keep = Keep::for_seconds(30);
pub(super) const ONE: Keep = Keep::for_seconds(15);

#[derive(Default)]
pub struct Query;

#[Object(name = "IssuesQuery")]
impl Query {
    /// Newest updated first; pull requests are not in it. first is 1 to 100. A page can hold fewer than first rows and still have a next page (PRJ-031).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.issues"
    )]
    async fn issues(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(default_with = "Some(OpenOrClosed::Open)")] state: Option<OpenOrClosed>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<IssueConnection> {
        let target = forge::target(ctx, &repository).await?;
        let (size, place) = forge::paging(first.unwrap_or(DEFAULT_FIRST), after.as_deref(), MOST)?;
        let state = state.unwrap_or(OpenOrClosed::Open).filter();
        let request = github::list_request(&target.name, state, place.page);
        let keep = LIST.fresh(fresh.unwrap_or(false));
        let reply = forge::read(ctx, &target, request, keep)
            .await?
            .ok_or_else(|| missing("repository"))?;
        let issues = forge::parsed(&reply, github::list_parse)?;
        let full = match reply.response().body {
            Body::Read(body) => entries(body) >= GITHUB_PAGE,
            Body::TooLarge => false,
        };
        let cut = cut(place, size, issues.len(), full);
        let (edges, page_info) = forge::page(issues, cut, |(cursor, found)| IssueEdge {
            cursor,
            node: issue(found, target.repository),
        });
        Ok(IssueConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }

    /// Null when the repository has no issue with this number, or the number is a pull request.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.issue"
    )]
    async fn issue(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        number: i32,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<Option<IssueView>> {
        let target = forge::target(ctx, &repository).await?;
        let number = forge::number(number)?;
        let request = github::one_request(&target.name, number);
        let keep = ONE.fresh(fresh.unwrap_or(false));
        let Some(reply) = forge::read(ctx, &target, request, keep).await? else {
            return Ok(None);
        };
        let found = forge::parsed(&reply, github::one_parse)?;
        if matches!(reply.response().body, Body::Read(body) if is_pull_request(body)) {
            return Ok(None);
        }
        Ok(Some(issue(found, target.repository)))
    }

    /// Oldest first. number is an issue or a pull request. first is 1 to 100.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.issueComments"
    )]
    async fn issue_comments(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        number: i32,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<CommentConnection> {
        let target = forge::target(ctx, &repository).await?;
        let number = forge::number(number)?;
        let (size, place) = forge::paging(first.unwrap_or(DEFAULT_FIRST), after.as_deref(), MOST)?;
        let request = comments::request(&target.name, number, place.page);
        let keep = LIST.fresh(fresh.unwrap_or(false));
        let reply = forge::read(ctx, &target, request, keep)
            .await?
            .ok_or_else(|| missing("issue or pull request"))?;
        let found = forge::parsed(&reply, comments::parse)?;
        let cut = cut(place, size, found.len(), found.len() >= GITHUB_PAGE);
        let (edges, page_info) = forge::page(found, cut, |(cursor, found)| CommentEdge {
            cursor,
            node: comment(found),
        });
        Ok(CommentConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }
}
