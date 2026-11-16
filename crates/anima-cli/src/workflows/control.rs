//! `anima workflow rerun` and `anima workflow cancel`: run a workflow run again, or only its
//! failed jobs, or cancel it, through the hub (PRJ-029, PRJ-034). Each reads the run first,
//! shows its commit in the question and sends that commit, so a run that changed is refused.

use serde_json::Value;

use super::documents::{
    WorkflowRunCancel, WorkflowRunGet, WorkflowRunRerun, WorkflowRunRerunFailed,
    workflow_run_cancel, workflow_run_get, workflow_run_rerun, workflow_run_rerun_failed,
};
use super::render;
use crate::failure::{Failure, Hint};
use crate::forge::render::{flag, number, text};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct RerunArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The run's ID, as `anima workflow runs` shows it.
    run: String,
    /// Run only the failed jobs again.
    #[arg(long)]
    failed: bool,
}

#[derive(Debug, clap::Args)]
pub struct CancelArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The run's ID, as `anima workflow runs` shows it.
    run: String,
}

/// What to do to the run.
#[derive(Clone, Copy)]
enum Control {
    Rerun,
    RerunFailed,
    Cancel,
}

impl Control {
    /// The `WorkflowRun` field that says the hub would take it.
    const fn allowed_by(self) -> &'static str {
        match self {
            Self::Rerun => "rerunnable",
            Self::RerunFailed => "failedJobsRerunnable",
            Self::Cancel => "cancellable",
        }
    }

    const fn question(self) -> &'static str {
        match self {
            Self::Rerun => "Run again every job of",
            Self::RerunFailed => "Run again the failed jobs of",
            Self::Cancel => "Cancel",
        }
    }

    const fn refusal(self) -> &'static str {
        match self {
            Self::Rerun => "it is not finished, or it is from a fork (PRO-013)",
            Self::RerunFailed => {
                "it is not finished, nothing in it failed, or it is from a fork (PRO-013)"
            }
            Self::Cancel => "it is not queued or running",
        }
    }
}

impl RerunArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), then prints the run or, with `--json`, the
    /// K1 `WorkflowRun`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown run; [`Failure::Conflict`] for a run that cannot
    /// run again; [`Failure::Declined`] when the operator says no; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let control = if self.failed {
            Control::RerunFailed
        } else {
            Control::Rerun
        };
        apply(invocation, &self.repository, &self.run, control).await
    }
}

impl CancelArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), then prints the run or, with `--json`, the
    /// K1 `WorkflowRun`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown run; [`Failure::Conflict`] for a run that is not
    /// running; [`Failure::Declined`] when the operator says no; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        apply(invocation, &self.repository, &self.run, Control::Cancel).await
    }
}

async fn apply(
    invocation: &Invocation,
    name: &str,
    run_id: &str,
    control: Control,
) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let repository = resolve(&hub, name).await?;
    let run = read(&hub, &repository, name, run_id).await?;
    if !flag(&run, control.allowed_by()) {
        return Err(Failure::Conflict {
            message: format!(
                "Run {run_id} cannot be changed so: {}. Nothing was sent.",
                control.refusal()
            ),
        });
    }
    let head = text(&run, "headSha").to_owned();
    invocation.prompter.confirm(&format!(
        "{} run {run_id} ({} #{}) on {}, commit {} \"{}\"?",
        control.question(),
        text(&run, "name"),
        number(&run, "runNumber"),
        text(&run, "branch"),
        head.get(..7).unwrap_or(&head),
        text(&run, "commitTitle"),
    ))?;
    let changed = send(&hub, (repository, run_id.to_owned(), head), control).await?;
    invocation.output.data(&changed, render::run)
}

/// The run as the hub reads it from GitHub now, never from its cache.
async fn read(hub: &Hub, repository: &str, name: &str, run_id: &str) -> Result<Value, Failure> {
    let variables = workflow_run_get::Variables {
        repository: repository.to_owned(),
        run_id: run_id.to_owned(),
        fresh: Some(true),
    };
    let run = hub
        .run::<WorkflowRunGet>(variables)
        .await?
        .workflow_run
        .ok_or_else(|| Failure::NotFound {
            message: format!("{name} has no run {run_id}."),
            hint: Some(Hint::run(format!("anima workflow runs {name}"))),
        })?;
    Ok(serde_json::to_value(&run).unwrap_or_default())
}

/// Sends the change for the run `(repository, run, head)`; the answer as JSON.
async fn send(
    hub: &Hub,
    (repository, run_id, expected_head_sha): (String, String, String),
    control: Control,
) -> Result<Value, Failure> {
    let changed = match control {
        Control::Rerun => {
            let variables = workflow_run_rerun::Variables {
                repository,
                run_id,
                expected_head_sha,
            };
            let run = hub.run::<WorkflowRunRerun>(variables).await?;
            serde_json::to_value(run.rerun_workflow_run)
        }
        Control::RerunFailed => {
            let variables = workflow_run_rerun_failed::Variables {
                repository,
                run_id,
                expected_head_sha,
            };
            let run = hub.run::<WorkflowRunRerunFailed>(variables).await?;
            serde_json::to_value(run.rerun_failed_workflow_jobs)
        }
        Control::Cancel => {
            let variables = workflow_run_cancel::Variables {
                repository,
                run_id,
                expected_head_sha,
            };
            let run = hub.run::<WorkflowRunCancel>(variables).await?;
            serde_json::to_value(run.cancel_workflow_run)
        }
    };
    Ok(changed.unwrap_or_default())
}
