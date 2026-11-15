//! `anima workflow start <repository> <workflow>`: starts a workflow by hand on a branch, with
//! named inputs, through the hub (PRJ-029, PRJ-035). The question names the inputs, never their
//! values, and nothing printed after it holds a value (LOG-007).

use super::documents::{WorkflowList, WorkflowStart, workflow_list, workflow_start};
use super::inputs::{self, Input};
use crate::failure::{Failure, Hint};
use crate::forge::render::text;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::projects::repository::resolve;

type Workflow = workflow_list::WorkflowListWorkflows;

#[derive(Debug, clap::Args)]
pub struct StartArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The workflow's ID or its path, such as `.github/workflows/release.yml`.
    workflow: String,
    /// The branch to run it on.
    #[arg(long)]
    branch: String,
    /// A named input of the workflow, `name=value`; repeat for more (at most 25).
    #[arg(long = "input", value_name = "NAME=VALUE")]
    inputs: Vec<String>,
}

impl StartArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), then prints what GitHub accepted or, with
    /// `--json`, the K1 `WorkflowDispatch`. The run shows in `anima workflow runs` a moment
    /// later.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an input that is not `name=value`; [`Failure::NotFound`] for an
    /// unknown workflow; [`Failure::Conflict`] for a disabled one; [`Failure::Declined`] when
    /// the operator says no; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let inputs = inputs::parse(&self.inputs)?;
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let workflow = find(&hub, &repository, &self.repository, &self.workflow).await?;
        let names: Vec<&str> = inputs.iter().map(|input| input.name.as_str()).collect();
        let with = if names.is_empty() {
            "no inputs".to_owned()
        } else {
            format!("the inputs {}", names.join(", "))
        };
        invocation.prompter.confirm(&format!(
            "Start {} ({}) on branch {} of {}, with {with}?",
            workflow.name, workflow.path, self.branch, self.repository
        ))?;
        let variables = workflow_start::Variables {
            input: workflow_start::DispatchWorkflowInput {
                repository,
                workflow_id: workflow.id.clone(),
                branch: self.branch.clone(),
                inputs: Some(inputs.into_iter().map(k1).collect()),
            },
        };
        let accepted = hub.run::<WorkflowStart>(variables).await?.dispatch_workflow;
        invocation.output.data(&accepted, |accepted| {
            let shown = serde_json::to_value(accepted).unwrap_or_default();
            let mut table = Table::record();
            table.pair("Workflow", format!("{} ({})", workflow.name, workflow.path));
            table.pair("Branch", text(&shown, "branch"));
            table.pair("Accepted", text(&shown, "acceptedAt"));
            table
        })?;
        invocation.output.note(&format!(
            "The run shows in `anima workflow runs {} --branch {}` in a moment.",
            self.repository, self.branch
        ));
        Ok(())
    }
}

fn k1(input: Input) -> workflow_start::WorkflowInputValue {
    workflow_start::WorkflowInputValue {
        name: input.name,
        value: input.value,
    }
}

/// The workflow `wanted` names, by its ID or its exact path; one that is disabled is refused.
async fn find(hub: &Hub, repository: &str, name: &str, wanted: &str) -> Result<Workflow, Failure> {
    let variables = workflow_list::Variables {
        repository: repository.to_owned(),
        fresh: Some(true),
    };
    let workflows = hub.run::<WorkflowList>(variables).await?.workflows;
    let workflow = workflows
        .into_iter()
        .find(|workflow| workflow.id == wanted || workflow.path == wanted)
        .ok_or_else(|| Failure::NotFound {
            message: format!("{name} has no workflow with the ID or path {wanted}."),
            hint: Some(Hint::run(format!("anima workflow list {name}"))),
        })?;
    if !workflow.active {
        return Err(Failure::Conflict {
            message: format!(
                "{} is disabled on GitHub, so it cannot start. Nothing was sent.",
                workflow.path
            ),
        });
    }
    Ok(workflow)
}
