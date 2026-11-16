//! What a change the hub sent to GitHub leaves behind: one audit entry and one event, whatever
//! came of it (PRJ-028; K-E1 Section 13.6). A change that was not sent leaves nothing. Pure.
//!
//! The entry names the function, the repository's checked `owner/name`, the target by its
//! number, the outcome and the numbers, commit, method, branch or context that fit. Never a
//! title, a body, a label, an input value or GitHub's reply (LOG-007). The event's words name
//! the project by its slug, which the hub made.

use serde_json::{Map, Value, json};
use source_exchange::{
    Change, CloseReason, IssueState, MergeMethod, Receipt, Refusal, Settled, StatusState,
};

use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::RepositoryId,
};

/// The entry of `change`, sent to `repository` of the project `slug`, that came to `settled`.
#[must_use]
pub fn entry(
    change: &Change,
    (repository, slug): (RepositoryId, &str),
    settled: &Settled,
) -> TrailEntry {
    let target = target(change, settled);
    let mut record = Map::new();
    record.insert("function".to_owned(), json!(change.function()));
    record.insert(
        "repository".to_owned(),
        json!(change.repository().to_string()),
    );
    record.insert("target".to_owned(), json!(target));
    record.insert("outcome".to_owned(), json!(outcome(settled)));
    match settled {
        Settled::Refused(refusal) => {
            record.insert("refusal".to_owned(), json!(refusal_kind(*refusal)));
        }
        Settled::Limited { until } => {
            record.insert("until".to_owned(), json!(until));
        }
        Settled::Done(Receipt::Created { number }) => {
            record.insert("created".to_owned(), json!(number));
        }
        Settled::Done(Receipt::Merged { sha }) => {
            record.insert("mergeSha".to_owned(), json!(sha.as_str()));
        }
        Settled::Done(Receipt::Made) | Settled::Unknown => {}
    }
    details(change, &mut record);
    TrailEntry {
        source: Source::Hub,
        severity: match settled {
            Settled::Done(_) => Severity::Info,
            _ => Severity::Warning,
        },
        device: None,
        message: format!(
            "Project {slug}: {} on {target}, {}.",
            change.function(),
            said(settled)
        ),
        action: action(change),
        target: AuditTarget::Repository(repository),
        change: Value::Object(record),
    }
}

/// The audit action of each function (K-E1 Section 13.6).
#[must_use]
pub const fn action(change: &Change) -> AuditAction {
    match change {
        Change::CreateIssue { .. }
        | Change::AddIssueComment { .. }
        | Change::SetCommitStatus { .. } => AuditAction::Create,
        Change::SetIssueOpen { .. } => AuditAction::Update,
        Change::MergePullRequest { .. } => AuditAction::Merge,
        Change::RerunWorkflowRun { .. }
        | Change::RerunFailedWorkflowJobs { .. }
        | Change::DispatchWorkflow { .. } => AuditAction::Start,
        Change::CancelWorkflowRun { .. } => AuditAction::Cancel,
    }
}

/// `done`, `refused`, `limited` or `unknown`.
#[must_use]
pub const fn outcome(settled: &Settled) -> &'static str {
    match settled {
        Settled::Done(_) => "done",
        Settled::Refused(_) => "refused",
        Settled::Limited { .. } => "limited",
        Settled::Unknown => "unknown",
    }
}

/// The event's words for the outcome.
const fn said(settled: &Settled) -> &'static str {
    match settled {
        Settled::Done(_) => "done",
        Settled::Refused(_) => "refused by GitHub",
        Settled::Limited { .. } => "refused: GitHub limits requests",
        Settled::Unknown => "sent with no answer from GitHub: it may have been made",
    }
}

const fn refusal_kind(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::KeyRefused => "key_refused",
        Refusal::NotPermitted => "not_permitted",
        Refusal::NotFound => "not_found",
        Refusal::HeadMoved => "head_moved",
        Refusal::NotMergeable => "not_mergeable",
        Refusal::Invalid => "invalid",
        Refusal::Gone => "gone",
        Refusal::Conflict => "conflict",
        Refusal::Unexplained => "unexplained",
    }
}

/// What the change was made to, by number: `issue#3`, `pull_request#12`, `run#…`,
/// `workflow#…` or `commit#…`. A new issue is `issue#new` until GitHub gave its number.
#[must_use]
pub fn target(change: &Change, settled: &Settled) -> String {
    match change {
        Change::CreateIssue { .. } => match settled {
            Settled::Done(Receipt::Created { number }) => format!("issue#{number}"),
            _ => "issue#new".to_owned(),
        },
        Change::AddIssueComment { number, .. } | Change::SetIssueOpen { number, .. } => {
            format!("issue#{number}")
        }
        Change::MergePullRequest { number, .. } => format!("pull_request#{number}"),
        Change::RerunWorkflowRun { run, .. }
        | Change::RerunFailedWorkflowJobs { run, .. }
        | Change::CancelWorkflowRun { run, .. } => format!("run#{run}"),
        Change::DispatchWorkflow { workflow, .. } => format!("workflow#{workflow}"),
        Change::SetCommitStatus { sha, .. } => format!("commit#{sha}"),
    }
}

/// The checked parts that fit the function. No text the operator wrote but a branch name and
/// a status context, which the parts bound; of a start's inputs only their count.
fn details(change: &Change, record: &mut Map<String, Value>) {
    let fields = match change {
        Change::CreateIssue { labels, .. } => json!({ "labels": labels.as_slice().len() }),
        Change::AddIssueComment { number, .. } => json!({ "number": number.get() }),
        Change::SetIssueOpen { number, state, .. } => {
            json!({ "number": number.get(), "open": *state == IssueState::Open,
                "reason": reason(*state) })
        }
        Change::MergePullRequest {
            number,
            head,
            method,
            ..
        } => json!({ "number": number.get(), "head": head.as_str(),
            "method": merge_method(*method) }),
        Change::RerunWorkflowRun { run, .. }
        | Change::RerunFailedWorkflowJobs { run, .. }
        | Change::CancelWorkflowRun { run, .. } => json!({ "run": run.get() }),
        Change::DispatchWorkflow {
            workflow,
            branch,
            inputs,
            ..
        } => json!({ "workflow": workflow.as_str(), "branch": branch.as_str(),
            "inputs": inputs.as_slice().len() }),
        Change::SetCommitStatus {
            sha,
            state,
            context,
            ..
        } => json!({ "sha": sha.as_str(), "context": context.as_str(),
            "state": status_state(*state) }),
    };
    if let Value::Object(fields) = fields {
        record.extend(fields);
    }
}

const fn reason(state: IssueState) -> Option<&'static str> {
    match state {
        IssueState::Closed(Some(CloseReason::Completed)) => Some("completed"),
        IssueState::Closed(Some(CloseReason::NotPlanned)) => Some("not_planned"),
        IssueState::Open | IssueState::Closed(None) => None,
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
