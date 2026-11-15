//! The run controls and a workflow started by hand (K1; OD-E14, PRJ-034, PRJ-035): run a run
//! again, run its failed jobs again, cancel it, and start a workflow on a branch.
//!
//! Each checks its input into the parts of a [`Change`] first, then the repository, then reads
//! GitHub: the run, never from the cache, whose state the rule of [`controls`] judges, or the
//! repository's workflows, as the cache holds them, of which the one named must be active. A refusal there sends nothing and records
//! nothing. The change is sent once and recorded by [`forge::make`]; a run is read back from
//! GitHub. Each makes code run with the repository's secrets or stops it: login or browser
//! token only (D-23, R-100; K-E1 Section 13.5).

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use provider_github::live::{
    runs::{self as github, Run},
    workflows::{self as github_workflows, Workflow},
};
use source_exchange::{Change, CommitSha, RepositoryName, RunId, WorkflowId};

use super::{
    controls::{self, Control, RunState},
    dispatch::{self, DispatchWorkflowInput, Found, WorkflowDispatch},
    graphql::{ONE, WORKFLOWS},
    views::{RunView, run},
};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, invalid, schema_dependency},
    clock::Clock,
    engine::sources::Keep,
    fleet::access::{Authenticated, operator_only},
    work::projects::forge::{self, Asked, Subject, Target, failure, missing},
};

/// The repository, the run and the head commit the operator saw, as K1 gives them.
type Asking<'a> = (&'a ID, &'a ID, &'a str);

#[derive(Default)]
pub struct Mutation;

#[Object(name = "WorkflowsMutation")]
impl Mutation {
    /// Runs every job again. CONFLICT when it is not rerunnable or expectedHeadSha is not its head commit (PRJ-034). Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.rerunWorkflowRun"
    )]
    async fn rerun_workflow_run(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        run_id: ID,
        expected_head_sha: String,
    ) -> Result<RunView> {
        operator_only(ctx, "rerunWorkflowRun")?;
        let asking = (&repository, &run_id, expected_head_sha.as_str());
        control(ctx, asking, Control::Rerun).await
    }

    /// Runs only the failed jobs again. CONFLICT as rerunWorkflowRun. Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.rerunFailedWorkflowJobs"
    )]
    async fn rerun_failed_workflow_jobs(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        run_id: ID,
        expected_head_sha: String,
    ) -> Result<RunView> {
        operator_only(ctx, "rerunFailedWorkflowJobs")?;
        let asking = (&repository, &run_id, expected_head_sha.as_str());
        control(ctx, asking, Control::RerunFailedJobs).await
    }

    /// CONFLICT when it is not cancellable or expectedHeadSha is not its head commit. Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.cancelWorkflowRun"
    )]
    async fn cancel_workflow_run(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        run_id: ID,
        expected_head_sha: String,
    ) -> Result<RunView> {
        operator_only(ctx, "cancelWorkflowRun")?;
        let asking = (&repository, &run_id, expected_head_sha.as_str());
        control(ctx, asking, Control::Cancel).await
    }

    /// Starts a workflow by hand on a branch (PRJ-035). VALIDATION when GitHub refuses the branch or the inputs; CONFLICT when the workflow is not active. Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn dispatch_workflow(
        &self,
        ctx: &Context<'_>,
        input: DispatchWorkflowInput,
    ) -> Result<WorkflowDispatch> {
        operator_only(ctx, "dispatchWorkflow")?;
        let start = dispatch::start(&input).map_err(|errors| invalid(&errors))?;
        let target = forge::target(ctx, &input.repository).await?;
        let repository = named(&target)?;
        let workflows = workflows(ctx, &target).await?;
        let workflow = match dispatch::find(&workflows, &start.workflow) {
            Found::Active(workflow) => workflow.id,
            Found::Inactive => {
                return Err(coded(
                    "The workflow is not active on GitHub. Enable it there first.",
                    ErrorCode::Conflict,
                ));
            }
            Found::Missing => return Err(missing("workflow")),
        };
        let branch = start.branch.as_str().to_owned();
        let change = Change::DispatchWorkflow {
            repository,
            // The number of the workflow found active, so the one checked is the one started.
            workflow: WorkflowId::parse(&workflow.to_string()).map_err(|_| failure::unusable())?,
            branch: start.branch,
            inputs: start.inputs,
        };
        forge::make(ctx, &target, &change, Subject::Workflow).await?;
        Ok(WorkflowDispatch {
            repository: ID(target.repository.to_string()),
            workflow_id: ID(workflow.to_string()),
            branch,
            accepted_at: now(ctx)?,
        })
    }
}

/// One run control: the run read fresh, the rule, the change, and the run read back.
async fn control(ctx: &Context<'_>, asking: Asking<'_>, control: Control) -> Result<RunView> {
    let (repository, run_id, expected) = asking;
    let (run_id, expected) = parts(run_id, expected)?;
    let target = forge::target(ctx, repository).await?;
    let repository = named(&target)?;
    // Never a held answer, not even a stale one: the rule acts on it (K-E1 Section 13.3).
    let found = one(ctx, &target, run_id, Keep::never())
        .await?
        .ok_or_else(|| missing("run"))?;
    controls::decide(control, &RunState::of(&found), &expected, now(ctx)?)
        .map_err(|barred| coded(barred.to_string(), ErrorCode::Conflict))?;
    let change = match control {
        Control::Rerun => Change::RerunWorkflowRun {
            repository,
            run: run_id,
        },
        Control::RerunFailedJobs => Change::RerunFailedWorkflowJobs {
            repository,
            run: run_id,
        },
        Control::Cancel => Change::CancelWorkflowRun {
            repository,
            run: run_id,
        },
    };
    forge::make(ctx, &target, &change, Subject::Run).await?;
    let after = one(ctx, &target, run_id, ONE.fresh(true))
        .await
        .ok()
        .flatten();
    let after = after.ok_or_else(failure::unread)?;
    Ok(run(
        after,
        Asked {
            target,
            fresh: true,
        },
    ))
}

/// The run and the head commit, or one `VALIDATION` error naming each that is not one.
fn parts(run_id: &ID, expected: &str) -> Result<(RunId, CommitSha)> {
    let run_id = RunId::parse(run_id.as_str())
        .map_err(|_| FieldError::new("runId", "Give GitHub's number, as digits."));
    let expected = CommitSha::parse(expected).map_err(|_| {
        FieldError::new(
            "expectedHeadSha",
            "Give the run's head commit as 40 hexadecimal digits.",
        )
    });
    match (run_id, expected) {
        (Ok(run_id), Ok(expected)) => Ok((run_id, expected)),
        (run_id, expected) => Err(invalid(
            &run_id
                .err()
                .into_iter()
                .chain(expected.err())
                .collect::<Vec<_>>(),
        )),
    }
}

/// Run `run` as GitHub has it now, read as `keep` says; `None` when GitHub has no such run.
async fn one(ctx: &Context<'_>, target: &Target, run: RunId, keep: Keep) -> Result<Option<Run>> {
    let request = github::one_request(&target.name, run.get());
    let Some(reply) = forge::read(ctx, target, request, keep).await? else {
        return Ok(None);
    };
    forge::parsed(&reply, github::one_parse).map(Some)
}

/// The repository's workflows, as the cache holds them (K-E1 Section 13.3). A held or stale
/// list is safe here: it names the workflow to start, and GitHub itself refuses a start of
/// one that is gone or turned off.
async fn workflows(ctx: &Context<'_>, target: &Target) -> Result<Vec<Workflow>> {
    let request = github_workflows::list_request(&target.name, 1);
    let reply = forge::read(ctx, target, request, WORKFLOWS)
        .await?
        .ok_or_else(|| missing("repository"))?;
    forge::parsed(&reply, github_workflows::list_parse)
}

/// The repository's stored name as the part a change holds.
///
/// # Errors
/// `CONFLICT` when the name is not one the hub sends (nothing is sent).
fn named(target: &Target) -> Result<RepositoryName> {
    RepositoryName::parse(&target.name).map_err(|_| failure::unnamed())
}

fn now(ctx: &Context<'_>) -> Result<chrono::DateTime<chrono::Utc>> {
    Ok(schema_dependency::<Arc<dyn Clock>>(ctx)?.now())
}
