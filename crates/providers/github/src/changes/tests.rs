//! The named changes as GitHub receives them, and what its replies mean (OD-E14, PRJ-027 to
//! PRJ-035). Endpoints, statuses and fields are those of GitHub's documentation, checked on
//! 2026-11-14 for API version 2022-11-28.

use source_exchange::{
    BranchName, Change, CommitSha, Inputs, IssueState, Labels, MergeMethod, Number, RepositoryName,
    RunId, StatusContext, StatusState, Text, Title, WorkflowId,
};

mod declaration;
mod paths;
mod render;
mod settle;

pub(super) const SHA: &str = "6dcb09b5b57875f334f61aebed695e2e4193db5e";

pub(super) fn repository() -> RepositoryName {
    RepositoryName::parse("octo-org/hello.world").unwrap()
}

pub(super) fn sha() -> CommitSha {
    CommitSha::parse(SHA).unwrap()
}

pub(super) fn number() -> Number {
    Number::new(1347).unwrap()
}

pub(super) fn run() -> RunId {
    RunId::parse("30433642").unwrap()
}

/// One ordinary change of each kind, in the order of [`Change`].
pub(super) fn every_change() -> Vec<Change> {
    vec![
        Change::CreateIssue {
            repository: repository(),
            title: Title::parse("Found a bug").unwrap(),
            body: None,
            labels: Labels::default(),
        },
        Change::AddIssueComment {
            repository: repository(),
            number: number(),
            body: Text::parse("Me too").unwrap(),
        },
        Change::SetIssueOpen {
            repository: repository(),
            number: number(),
            state: IssueState::Open,
        },
        Change::MergePullRequest {
            repository: repository(),
            number: number(),
            head: sha(),
            method: MergeMethod::Squash,
        },
        Change::RerunWorkflowRun {
            repository: repository(),
            run: run(),
        },
        Change::RerunFailedWorkflowJobs {
            repository: repository(),
            run: run(),
        },
        Change::CancelWorkflowRun {
            repository: repository(),
            run: run(),
        },
        Change::DispatchWorkflow {
            repository: repository(),
            workflow: WorkflowId::parse("build.yml").unwrap(),
            branch: BranchName::parse("main").unwrap(),
            inputs: Inputs::default(),
        },
        Change::SetCommitStatus {
            repository: repository(),
            sha: sha(),
            state: StatusState::Success,
            context: StatusContext::parse("anima/build").unwrap(),
            description: None,
            target_url: None,
        },
    ]
}
