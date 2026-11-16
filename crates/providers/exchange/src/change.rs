//! The named changes a source may make, and how a provider says what one means on the wire
//! (PRJ-027 to PRJ-035, OD-E14).
//!
//! [`crate::Method`] keeps its single `GET`: a sync can still not express a write. A write is a
//! [`Change`], a closed set built only from checked parts. A provider's [`Changes::render`]
//! turns one into a [`Rendered`] request whose path is made of fixed segments and checked parts
//! only, and whose verb the variant fixes; [`Changes::settle`] reads the reply into a closed
//! [`Settled`], never quoting it.

mod rendered;
mod settled;

pub use rendered::{ChangePath, Fixed, Rendered};
pub use settled::{ChangeReply, Limit, Receipt, Refusal, Settled};

use crate::parts::{
    BranchName, CommitSha, Inputs, IssueState, Labels, MergeMethod, Number, RepositoryName, RunId,
    StatusContext, StatusDescription, StatusState, TargetUrl, Text, Title, WorkflowId,
};

/// One named change to a repository. There is no variant that carries a path or a verb.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    CreateIssue {
        repository: RepositoryName,
        title: Title,
        body: Option<Text>,
        labels: Labels,
    },
    /// On an issue or a pull request.
    AddIssueComment {
        repository: RepositoryName,
        number: Number,
        body: Text,
    },
    SetIssueOpen {
        repository: RepositoryName,
        number: Number,
        state: IssueState,
    },
    /// Merges only while the head is still `head`.
    MergePullRequest {
        repository: RepositoryName,
        number: Number,
        head: CommitSha,
        method: MergeMethod,
    },
    RerunWorkflowRun {
        repository: RepositoryName,
        run: RunId,
    },
    RerunFailedWorkflowJobs {
        repository: RepositoryName,
        run: RunId,
    },
    CancelWorkflowRun {
        repository: RepositoryName,
        run: RunId,
    },
    DispatchWorkflow {
        repository: RepositoryName,
        workflow: WorkflowId,
        branch: BranchName,
        inputs: Inputs,
    },
    SetCommitStatus {
        repository: RepositoryName,
        sha: CommitSha,
        state: StatusState,
        context: StatusContext,
        description: Option<StatusDescription>,
        target_url: Option<TargetUrl>,
    },
}

/// The HTTP verbs a change may use. There is no `DELETE`: no named change removes anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Post,
    Put,
    Patch,
}

impl Change {
    /// The verb of this change, fixed here for each variant so a provider cannot choose one.
    #[must_use]
    pub const fn verb(&self) -> Verb {
        match self {
            Self::SetIssueOpen { .. } => Verb::Patch,
            Self::MergePullRequest { .. } => Verb::Put,
            Self::CreateIssue { .. }
            | Self::AddIssueComment { .. }
            | Self::RerunWorkflowRun { .. }
            | Self::RerunFailedWorkflowJobs { .. }
            | Self::CancelWorkflowRun { .. }
            | Self::DispatchWorkflow { .. }
            | Self::SetCommitStatus { .. } => Verb::Post,
        }
    }

    /// The name of the function in the audit log (PRJ-028), for example `merge_pull_request`.
    #[must_use]
    pub const fn function(&self) -> &'static str {
        match self {
            Self::CreateIssue { .. } => "create_issue",
            Self::AddIssueComment { .. } => "add_issue_comment",
            Self::SetIssueOpen { .. } => "set_issue_open",
            Self::MergePullRequest { .. } => "merge_pull_request",
            Self::RerunWorkflowRun { .. } => "rerun_workflow_run",
            Self::RerunFailedWorkflowJobs { .. } => "rerun_failed_workflow_jobs",
            Self::CancelWorkflowRun { .. } => "cancel_workflow_run",
            Self::DispatchWorkflow { .. } => "dispatch_workflow",
            Self::SetCommitStatus { .. } => "set_commit_status",
        }
    }

    /// The repository the change is made in.
    #[must_use]
    pub const fn repository(&self) -> &RepositoryName {
        match self {
            Self::CreateIssue { repository, .. }
            | Self::AddIssueComment { repository, .. }
            | Self::SetIssueOpen { repository, .. }
            | Self::MergePullRequest { repository, .. }
            | Self::RerunWorkflowRun { repository, .. }
            | Self::RerunFailedWorkflowJobs { repository, .. }
            | Self::CancelWorkflowRun { repository, .. }
            | Self::DispatchWorkflow { repository, .. }
            | Self::SetCommitStatus { repository, .. } => repository,
        }
    }
}

/// What a provider that can make changes gives the engine: two pure functions.
#[derive(Clone, Copy, Debug)]
pub struct Changes {
    /// The request that makes `change`, below the source's `api_base`.
    pub render: fn(change: &Change) -> Rendered,
    /// What the reply to the rendered `change` means. Fixed outcomes, never the reply's text.
    pub settle: fn(change: &Change, reply: &ChangeReply<'_>) -> Settled,
}
