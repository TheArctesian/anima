//! The workflows of a repository (PRJ-031).

use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::{get, read};

/// GitHub's workflow `state`, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowState {
    Active,
    Deleted,
    DisabledFork,
    DisabledInactivity,
    DisabledManually,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    /// For example `.github/workflows/ci.yml`.
    pub path: String,
    pub state: WorkflowState,
    pub url: String,
}

#[derive(Deserialize)]
struct RawWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
    html_url: String,
}

#[derive(Deserialize)]
struct WorkflowList {
    workflows: Vec<RawWorkflow>,
}

/// `GET /repos/{o}/{r}/actions/workflows`, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn list_request(repository: &str, page: u32) -> Result<Request, Problem> {
    get(repository, "/actions/workflows", &[], Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of workflows.
pub fn list_parse(response: &Response<'_>) -> Result<Vec<Workflow>, Problem> {
    let list: WorkflowList = read(response, "workflows")?;
    Ok(list
        .workflows
        .into_iter()
        .map(|workflow| Workflow {
            id: workflow.id,
            name: workflow.name,
            path: workflow.path,
            state: match workflow.state.as_str() {
                "active" => WorkflowState::Active,
                "deleted" => WorkflowState::Deleted,
                "disabled_fork" => WorkflowState::DisabledFork,
                "disabled_inactivity" => WorkflowState::DisabledInactivity,
                "disabled_manually" => WorkflowState::DisabledManually,
                _ => WorkflowState::Other,
            },
            url: workflow.html_url,
        })
        .collect())
}
