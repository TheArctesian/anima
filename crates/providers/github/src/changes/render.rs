//! The request for each named change: its path from fixed segments and checked parts only, and
//! its JSON body (OD-E14). A branch, a title, a body or an input goes into the JSON body, never
//! into the path. Endpoints, statuses and body fields are GitHub's REST API version 2022-11-28.

use serde_json::{Map, Value, json};
use source_exchange::{
    Change, ChangePath, CloseReason, Fixed, Inputs, IssueState, Label, Labels, MergeMethod,
    Rendered, StatusState, Text,
};

const REPOS: Fixed = Fixed::new("repos");
const ISSUES: Fixed = Fixed::new("issues");
const COMMENTS: Fixed = Fixed::new("comments");
const PULLS: Fixed = Fixed::new("pulls");
const MERGE: Fixed = Fixed::new("merge");
const ACTIONS: Fixed = Fixed::new("actions");
const RUNS: Fixed = Fixed::new("runs");
const RERUN: Fixed = Fixed::new("rerun");
const RERUN_FAILED_JOBS: Fixed = Fixed::new("rerun-failed-jobs");
const CANCEL: Fixed = Fixed::new("cancel");
const WORKFLOWS: Fixed = Fixed::new("workflows");
const DISPATCHES: Fixed = Fixed::new("dispatches");
const STATUSES: Fixed = Fixed::new("statuses");

/// The request that makes `change`, below `https://api.github.com`.
pub(crate) fn render(change: &Change) -> Rendered {
    Rendered::new(change, path(change), body(change))
}

/// `/repos/{owner}/{repo}/` and the endpoint of the change.
fn path(change: &Change) -> ChangePath {
    let repository = ChangePath::new(REPOS).repository(change.repository());
    match change {
        Change::CreateIssue { .. } => repository.fixed(ISSUES),
        Change::AddIssueComment { number, .. } => {
            repository.fixed(ISSUES).number(*number).fixed(COMMENTS)
        }
        Change::SetIssueOpen { number, .. } => repository.fixed(ISSUES).number(*number),
        Change::MergePullRequest { number, .. } => {
            repository.fixed(PULLS).number(*number).fixed(MERGE)
        }
        Change::RerunWorkflowRun { run, .. } => {
            repository.fixed(ACTIONS).fixed(RUNS).run(*run).fixed(RERUN)
        }
        Change::RerunFailedWorkflowJobs { run, .. } => repository
            .fixed(ACTIONS)
            .fixed(RUNS)
            .run(*run)
            .fixed(RERUN_FAILED_JOBS),
        Change::CancelWorkflowRun { run, .. } => repository
            .fixed(ACTIONS)
            .fixed(RUNS)
            .run(*run)
            .fixed(CANCEL),
        Change::DispatchWorkflow { workflow, .. } => repository
            .fixed(ACTIONS)
            .fixed(WORKFLOWS)
            .workflow(workflow)
            .fixed(DISPATCHES),
        Change::SetCommitStatus { sha, .. } => repository.fixed(STATUSES).sha(sha),
    }
}

/// The JSON body. `None` for the run controls: they send none. An optional field that is not
/// set is left out, so GitHub keeps its default.
fn body(change: &Change) -> Option<Value> {
    match change {
        Change::CreateIssue {
            title,
            body,
            labels,
            ..
        } => Some(new_issue(
            title.as_str(),
            body.as_ref().map(Text::as_str),
            labels,
        )),
        Change::AddIssueComment { body, .. } => Some(json!({ "body": body.as_str() })),
        Change::SetIssueOpen { state, .. } => Some(issue_state(*state)),
        Change::MergePullRequest { head, method, .. } => Some(json!({
            "merge_method": merge_method(*method),
            "sha": head.as_str(),
        })),
        Change::RerunWorkflowRun { .. }
        | Change::RerunFailedWorkflowJobs { .. }
        | Change::CancelWorkflowRun { .. } => None,
        Change::DispatchWorkflow { branch, inputs, .. } => Some(json!({
            "ref": branch.as_str(),
            "inputs": input_object(inputs),
        })),
        Change::SetCommitStatus {
            state,
            context,
            description,
            target_url,
            ..
        } => {
            let mut status = Map::new();
            status.insert("state".to_owned(), json!(status_state(*state)));
            status.insert("context".to_owned(), json!(context.as_str()));
            if let Some(description) = description {
                status.insert("description".to_owned(), json!(description.as_str()));
            }
            if let Some(url) = target_url {
                status.insert("target_url".to_owned(), json!(url.as_str()));
            }
            Some(Value::Object(status))
        }
    }
}

fn new_issue(title: &str, body: Option<&str>, labels: &Labels) -> Value {
    let mut issue = Map::new();
    issue.insert("title".to_owned(), json!(title));
    if let Some(body) = body {
        issue.insert("body".to_owned(), json!(body));
    }
    if !labels.as_slice().is_empty() {
        let names: Vec<&str> = labels.as_slice().iter().map(Label::as_str).collect();
        issue.insert("labels".to_owned(), json!(names));
    }
    Value::Object(issue)
}

/// Open, or closed with the reason when one is given. Anima never writes `duplicate`
/// (decisions item 9).
fn issue_state(state: IssueState) -> Value {
    match state {
        IssueState::Open => json!({ "state": "open" }),
        IssueState::Closed(None) => json!({ "state": "closed" }),
        IssueState::Closed(Some(reason)) => json!({
            "state": "closed",
            "state_reason": match reason {
                CloseReason::Completed => "completed",
                CloseReason::NotPlanned => "not_planned",
            },
        }),
    }
}

const fn merge_method(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Merge => "merge",
        MergeMethod::Squash => "squash",
        MergeMethod::Rebase => "rebase",
    }
}

const fn status_state(state: StatusState) -> &'static str {
    match state {
        StatusState::Pending => "pending",
        StatusState::Success => "success",
        StatusState::Failure => "failure",
        StatusState::Error => "error",
    }
}

/// The inputs as one object of strings, in the order given (decisions item 11).
fn input_object(inputs: &Inputs) -> Value {
    let object: Map<String, Value> = inputs
        .as_slice()
        .iter()
        .map(|(name, value)| (name.as_str().to_owned(), json!(value.as_str())))
        .collect();
    Value::Object(object)
}
