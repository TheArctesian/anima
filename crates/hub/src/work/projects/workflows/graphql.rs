//! The workflow roots (K1): a repository's workflows, a page of its runs and one run, with the
//! jobs of a run when they are selected.

use async_graphql::{ComplexObject, Context, ID, Object, Result};
use provider_github::live::{
    runs::{self as github, RunFilter},
    workflows as github_workflows,
};
use source_exchange::{Body, Response};

use super::{
    jobs::{self, JobsView},
    views::{RunConnection, RunEdge, RunView, WorkflowRunStatus, WorkflowView, run, workflows},
};
use crate::{
    api_errors::{FieldError, invalid},
    engine::sources::Keep,
    fleet::access::Authenticated,
    work::projects::forge::{self, Asked, GITHUB_PAGE, cut, missing},
};

/// K1's default for `first`, and its most.
const DEFAULT_FIRST: i32 = 20;
const MOST: usize = 50;
/// A filtered run list ends at 1,000 results: 10 of GitHub's pages (decisions item 10).
const LAST_FILTERED_PAGE: u32 = 10;
/// The longest branch name GitHub takes, in bytes.
const BRANCH_BYTES: usize = 255;
/// How long each answer stays fresh (K-E1 Section 13.1).
const LIST: Keep = Keep::for_seconds(30);
const ONE: Keep = Keep::for_seconds(15);
const WORKFLOWS: Keep = Keep::for_seconds(300);
const JOBS: Keep = Keep::for_seconds(15);
const SETTLED_SECONDS: i64 = 600;

/// A branch to narrow the runs to: 1 to 255 bytes, no control character.
fn branch(branch: Option<&str>) -> Result<Option<&str>> {
    match branch {
        Some(name)
            if name.is_empty()
                || name.len() > BRANCH_BYTES
                || name.chars().any(char::is_control) =>
        {
            Err(invalid(&[FieldError::new(
                "branch",
                "Give a branch name of 1 to 255 bytes with no control character.",
            )]))
        }
        other => Ok(other),
    }
}

/// The jobs body has jobs and every one has finished.
fn jobs_settled(body: &[u8]) -> bool {
    let answer = Response {
        status: 200,
        body: Body::Read(body),
    };
    github::jobs_parse(&answer).is_ok_and(|found| jobs::settled(&found))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "WorkflowsQuery")]
impl Query {
    /// Every workflow of the repository, at most 100, by name (PRJ-035).
    #[graphql(guard = "Authenticated")]
    async fn workflows(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<Vec<WorkflowView>> {
        let target = forge::target(ctx, &repository).await?;
        let request = github_workflows::list_request(&target.name, 1);
        let keep = WORKFLOWS.fresh(fresh.unwrap_or(false));
        let reply = forge::read(ctx, &target, request, keep)
            .await?
            .ok_or_else(|| missing("repository"))?;
        Ok(workflows(forge::parsed(
            &reply,
            github_workflows::list_parse,
        )?))
    }

    /// Newest first. branch null: every branch. status null: every status. first is 1 to 50 (PRJ-031, PRJ-034).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.workflowRuns"
    )]
    async fn workflow_runs(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        branch: Option<String>,
        status: Option<WorkflowRunStatus>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<RunConnection> {
        let target = forge::target(ctx, &repository).await?;
        let filter = RunFilter {
            branch: self::branch(branch.as_deref())?,
            status: status.map(WorkflowRunStatus::filter),
        };
        let (size, place) = forge::paging(first.unwrap_or(DEFAULT_FIRST), after.as_deref(), MOST)?;
        let asked = Asked {
            target,
            fresh: fresh.unwrap_or(false),
        };
        let request = github::list_request(&asked.target.name, filter, place.page);
        let reply = forge::read(ctx, &asked.target, request, LIST.fresh(asked.fresh))
            .await?
            .ok_or_else(|| missing("repository"))?;
        let runs = forge::parsed(&reply, github::list_parse)?;
        let filtered = filter.branch.is_some() || filter.status.is_some();
        let full = runs.len() >= GITHUB_PAGE && !(filtered && place.page >= LAST_FILTERED_PAGE);
        let cut = cut(place, size, runs.len(), full);
        let (edges, page_info) = forge::page(runs, cut, |(cursor, found)| RunEdge {
            cursor,
            node: run(found, asked.clone()),
        });
        Ok(RunConnection {
            edges,
            page_info,
            read: forge::stamp(&reply),
        })
    }

    /// Null when the repository has no run with this number.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.workflowRun"
    )]
    async fn workflow_run(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        run_id: ID,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<Option<RunView>> {
        let target = forge::target(ctx, &repository).await?;
        let id = forge::digits(&run_id, "runId")?;
        let asked = Asked {
            target,
            fresh: fresh.unwrap_or(false),
        };
        let request = github::one_request(&asked.target.name, id);
        let Some(reply) = forge::read(ctx, &asked.target, request, ONE.fresh(asked.fresh)).await?
        else {
            return Ok(None);
        };
        Ok(Some(run(forge::parsed(&reply, github::one_parse)?, asked)))
    }
}

#[ComplexObject]
impl RunView {
    /// Costs one GitHub request for each run when selected.
    async fn jobs(&self, ctx: &Context<'_>) -> Result<Option<JobsView>> {
        let asked = &self.asked;
        let request = github::jobs_request(&asked.target.name, self.github_id, 1);
        let keep = JOBS
            .longer_when(SETTLED_SECONDS, jobs_settled)
            .fresh(asked.fresh);
        let Some(reply) = forge::read(ctx, &asked.target, request, keep).await? else {
            return Ok(None);
        };
        Ok(Some(jobs::count(&forge::parsed(
            &reply,
            github::jobs_parse,
        )?)))
    }
}
