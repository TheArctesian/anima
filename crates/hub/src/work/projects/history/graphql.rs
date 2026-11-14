//! The history roots (K1): a page of a repository's commits, its branches and its releases on
//! GitHub, with the checks of a commit when they are selected.

use async_graphql::{ComplexObject, Context, ID, Object, Result};
use provider_github::live::{commits as github_commits, refs as github_refs};

use super::{
    omitted,
    views::{
        BranchConnection, BranchEdge, CommitConnection, CommitEdge, CommitView, ReleaseConnection,
        ReleaseEdge, branch, commit, release,
    },
};
use crate::{
    engine::sources::Keep,
    fleet::access::Authenticated,
    work::projects::forge::{self, Asked, CommitChecksView, GITHUB_PAGE, cut, missing},
};

/// K1's default for `first` of each list; each takes at most 100.
const COMMITS_FIRST: i32 = 30;
const BRANCHES_FIRST: i32 = 100;
const RELEASES_FIRST: i32 = 20;
const MOST: usize = 100;
/// How long each answer stays fresh (K-E1 Section 13.1).
const COMMITS: Keep = Keep::for_seconds(60);
const BRANCHES: Keep = Keep::for_seconds(60);
const RELEASES: Keep = Keep::for_seconds(300);

#[derive(Default)]
pub struct Query;

#[Object(name = "HistoryQuery")]
impl Query {
    /// Newest first. ref is a branch name or a 40-digit commit; null: the default branch. first is 1 to 100.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.commits"
    )]
    async fn commits(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(name = "ref")] git_ref: Option<String>,
        #[graphql(default_with = "Some(COMMITS_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<CommitConnection> {
        let target = forge::target(ctx, &repository).await?;
        let (size, place) = forge::paging(first.unwrap_or(COMMITS_FIRST), after.as_deref(), MOST)?;
        let request = forge::checked(github_commits::list_request(
            &target.name,
            git_ref.as_deref(),
            place.page,
        ))?;
        let asked = Asked {
            target,
            fresh: fresh.unwrap_or(false),
        };
        let reply = forge::read(ctx, &asked.target, request, COMMITS.fresh(asked.fresh))
            .await?
            .ok_or_else(|| missing("branch or commit"))?;
        let commits = forge::parsed(&reply, github_commits::list_parse)?;
        let parents = forge::parsed(&reply, omitted::parents)?;
        let cut = cut(place, size, commits.len(), commits.len() >= GITHUB_PAGE);
        let rows = commits.into_iter().zip(parents).collect();
        let (edges, page_info) = forge::page(rows, cut, |(cursor, (found, parents))| CommitEdge {
            cursor,
            node: commit(found, parents, asked.clone()),
        });
        Ok(CommitConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }

    /// By name. first is 1 to 100.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.branches"
    )]
    async fn branches(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(default_with = "Some(BRANCHES_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<BranchConnection> {
        let target = forge::target(ctx, &repository).await?;
        let (size, place) = forge::paging(first.unwrap_or(BRANCHES_FIRST), after.as_deref(), MOST)?;
        let request = github_refs::branches_request(&target.name, place.page);
        let keep = BRANCHES.fresh(fresh.unwrap_or(false));
        let reply = forge::read(ctx, &target, request, keep)
            .await?
            .ok_or_else(|| missing("repository"))?;
        let branches = forge::parsed(&reply, github_refs::branches_parse)?;
        let cut = cut(place, size, branches.len(), branches.len() >= GITHUB_PAGE);
        let default_branch = target.default_branch.as_deref();
        let (edges, page_info) = forge::page(branches, cut, |(cursor, found)| BranchEdge {
            cursor,
            node: branch(found, default_branch),
        });
        Ok(BranchConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }

    /// Newest first. first is 1 to 100. Releases on GitHub, not releases of Anima.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.repositoryReleases"
    )]
    async fn repository_releases(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(default_with = "Some(RELEASES_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<ReleaseConnection> {
        let target = forge::target(ctx, &repository).await?;
        let (size, place) = forge::paging(first.unwrap_or(RELEASES_FIRST), after.as_deref(), MOST)?;
        let request = github_refs::releases_request(&target.name, place.page);
        let keep = RELEASES.fresh(fresh.unwrap_or(false));
        let reply = forge::read(ctx, &target, request, keep)
            .await?
            .ok_or_else(|| missing("repository"))?;
        let releases = forge::parsed(&reply, github_refs::releases_parse)?;
        let made = forge::parsed(&reply, omitted::made)?;
        let cut = cut(place, size, releases.len(), releases.len() >= GITHUB_PAGE);
        let rows = releases.into_iter().zip(made).collect();
        let (edges, page_info) = forge::page(rows, cut, |(cursor, (found, made))| ReleaseEdge {
            cursor,
            node: release(found, made),
        });
        Ok(ReleaseConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }
}

#[ComplexObject]
impl CommitView {
    /// Costs two GitHub requests for each commit when selected.
    async fn checks(&self, ctx: &Context<'_>) -> Result<Option<CommitChecksView>> {
        let asked = &self.asked;
        forge::commit_checks(ctx, &asked.target, &self.sha, asked.fresh).await
    }
}
