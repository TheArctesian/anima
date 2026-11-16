//! A workflow started by hand (PRJ-035; K-E1 Section 13.3): the K1 input and answer, the input
//! checked into the parts of a change, and the workflow it names found among the repository's
//! workflows. Pure.
//!
//! The operator types the inputs as name and value pairs and GitHub checks them; the hub parses
//! no YAML. A value goes into the change's JSON body and nowhere else: no error, audit entry,
//! event or log line holds one (LOG-007).

use async_graphql::{ID, InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::workflows::{Workflow, WorkflowState};
use source_exchange::{BranchName, InputName, InputValue, Inputs, PartError, WorkflowId};

use crate::api_errors::FieldError;

/// One named input of a workflow started by hand. The operator types the name; GitHub checks it (PRJ-035).
#[derive(InputObject)]
#[graphql(name = "WorkflowInputValue")]
pub struct WorkflowInputValue {
    /// 1 to 100 characters: letters, digits, - and _.
    name: String,
    /// At most 1,024 characters.
    value: String,
}

#[derive(InputObject)]
#[graphql(name = "DispatchWorkflowInput")]
pub struct DispatchWorkflowInput {
    pub repository: ID,
    pub workflow_id: ID,
    /// A branch of this repository.
    branch: String,
    /// At most 25.
    #[graphql(default_with = "Some(Vec::new())")]
    inputs: Option<Vec<WorkflowInputValue>>,
}

/// GitHub accepted the request; the run appears in workflowRuns a moment later.
#[derive(SimpleObject)]
#[graphql(name = "WorkflowDispatch")]
pub struct WorkflowDispatch {
    pub repository: ID,
    pub workflow_id: ID,
    pub branch: String,
    pub accepted_at: DateTime<Utc>,
}

/// The checked parts of a start: the workflow as the caller named it, the branch and the
/// inputs. Not `Debug`: the inputs hold the values.
pub struct Start {
    pub workflow: WorkflowId,
    pub branch: BranchName,
    pub inputs: Inputs,
}

/// The parts of `input`, or one field error for each that is not one. The errors are fixed
/// words: never a name or a value the caller gave.
///
/// # Errors
/// The field errors of `workflowId`, `branch` and `inputs`.
pub fn start(input: &DispatchWorkflowInput) -> Result<Start, Vec<FieldError>> {
    let workflow = WorkflowId::parse(input.workflow_id.as_str()).map_err(|_| {
        FieldError::new(
            "workflowId",
            "Give the workflow's number, or its file name ending .yml or .yaml.",
        )
    });
    let branch = BranchName::parse(&input.branch).map_err(|_| {
        FieldError::new(
            "branch",
            "Give a branch name git allows, of 1 to 255 bytes.",
        )
    });
    let inputs = inputs(input.inputs.as_deref().unwrap_or_default())
        .map_err(|refused| FieldError::new("inputs", words(refused)));
    match (workflow, branch, inputs) {
        (Ok(workflow), Ok(branch), Ok(inputs)) => Ok(Start {
            workflow,
            branch,
            inputs,
        }),
        (workflow, branch, inputs) => Err([workflow.err(), branch.err(), inputs.err()]
            .into_iter()
            .flatten()
            .collect()),
    }
}

fn inputs(given: &[WorkflowInputValue]) -> Result<Inputs, PartError> {
    let pairs = given
        .iter()
        .map(|input| {
            Ok((
                InputName::parse(&input.name)?,
                InputValue::parse(&input.value)?,
            ))
        })
        .collect::<Result<Vec<_>, PartError>>()?;
    Inputs::new(pairs)
}

const fn words(refused: PartError) -> &'static str {
    match refused {
        PartError::InputName => "Give each input a name of 1 to 100 letters, digits, - and _.",
        PartError::InputValue => {
            "Give each input a value of at most 1,024 characters with no NUL or control character \
             but tab and line breaks."
        }
        _ => "Give at most 25 inputs, each name once.",
    }
}

/// What the workflow named `workflow` is among `workflows`, the repository's.
#[derive(Debug, PartialEq, Eq)]
pub enum Found<'a> {
    Active(&'a Workflow),
    Inactive,
    Missing,
}

/// The workflow `workflow` names: by its number, or by the file name its path ends with.
#[must_use]
pub fn find<'a>(workflows: &'a [Workflow], workflow: &WorkflowId) -> Found<'a> {
    let named = workflow.as_str();
    let file = |path: &str| path.rsplit('/').next() == Some(named);
    let found = workflows
        .iter()
        .find(|candidate| candidate.id.to_string() == named || file(&candidate.path));
    match found {
        Some(found) if found.state == WorkflowState::Active => Found::Active(found),
        Some(_) => Found::Inactive,
        None => Found::Missing,
    }
}
