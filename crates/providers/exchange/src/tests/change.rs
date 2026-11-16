//! A named change has a fixed verb, and a rendered path made of its checked parts stays a
//! plain path the engine accepts (OD-E14, PRJ-027 to PRJ-035).

use proptest::prelude::*;
use serde_json::json;

use crate::{
    BranchName, Change, ChangePath, ChangeReply, Changes, CommitSha, Fixed, Inputs, IssueState,
    Labels, MergeMethod, Number, Receipt, RepositoryName, Request, RunId, Settled, StatusContext,
    StatusState, Text, Title, Verb, WorkflowId,
};

const REPOS: Fixed = Fixed::new("repos");
const ISSUES: Fixed = Fixed::new("issues");
const PULLS: Fixed = Fixed::new("pulls");
const MERGE: Fixed = Fixed::new("merge");
const ACTIONS: Fixed = Fixed::new("actions");
const RUNS: Fixed = Fixed::new("runs");
const RERUN: Fixed = Fixed::new("rerun");

fn repository() -> RepositoryName {
    RepositoryName::parse("octo-org/hello.world").unwrap()
}

fn sha() -> CommitSha {
    CommitSha::parse("6dcb09b5b57875f334f61aebed695e2e4193db5e").unwrap()
}

/// One change of each kind.
fn every_change(repository: &RepositoryName, number: Number, run: RunId) -> Vec<Change> {
    let repository = || repository.clone();
    vec![
        Change::CreateIssue {
            repository: repository(),
            title: Title::parse("t").unwrap(),
            body: None,
            labels: Labels::default(),
        },
        Change::AddIssueComment {
            repository: repository(),
            number,
            body: Text::parse("b").unwrap(),
        },
        Change::SetIssueOpen {
            repository: repository(),
            number,
            state: IssueState::Closed(None),
        },
        Change::MergePullRequest {
            repository: repository(),
            number,
            head: sha(),
            method: MergeMethod::Squash,
        },
        Change::RerunWorkflowRun {
            repository: repository(),
            run,
        },
        Change::RerunFailedWorkflowJobs {
            repository: repository(),
            run,
        },
        Change::CancelWorkflowRun {
            repository: repository(),
            run,
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

/// A provider's render, shaped like GitHub's, built only from the change's parts.
fn render(change: &Change) -> crate::Rendered {
    let below = ChangePath::new(REPOS).repository(change.repository());
    let path = match change {
        Change::CreateIssue { .. } => below.fixed(ISSUES),
        Change::AddIssueComment { number, .. } => below
            .fixed(ISSUES)
            .number(*number)
            .fixed(Fixed::new("comments")),
        Change::SetIssueOpen { number, .. } => below.fixed(ISSUES).number(*number),
        Change::MergePullRequest { number, .. } => below.fixed(PULLS).number(*number).fixed(MERGE),
        Change::RerunWorkflowRun { run, .. } => {
            below.fixed(ACTIONS).fixed(RUNS).run(*run).fixed(RERUN)
        }
        Change::RerunFailedWorkflowJobs { run, .. } => below
            .fixed(ACTIONS)
            .fixed(RUNS)
            .run(*run)
            .fixed(Fixed::new("rerun-failed-jobs")),
        Change::CancelWorkflowRun { run, .. } => below
            .fixed(ACTIONS)
            .fixed(RUNS)
            .run(*run)
            .fixed(Fixed::new("cancel")),
        Change::DispatchWorkflow { workflow, .. } => below
            .fixed(ACTIONS)
            .fixed(Fixed::new("workflows"))
            .workflow(workflow)
            .fixed(Fixed::new("dispatches")),
        Change::SetCommitStatus { sha, .. } => below.fixed(Fixed::new("statuses")).sha(sha),
    };
    crate::Rendered::new(change, path, Some(json!({})))
}

fn settle(_: &Change, reply: &ChangeReply<'_>) -> Settled {
    match reply.status {
        200..=299 => Settled::Done(Receipt::Made),
        _ => Settled::Unknown,
    }
}

#[test]
fn od_e14_each_change_has_a_fixed_verb_and_no_delete() {
    let changes = every_change(
        &repository(),
        Number::new(12).unwrap(),
        RunId::parse("30433642").unwrap(),
    );
    let verbs: Vec<Verb> = changes.iter().map(Change::verb).collect();
    let (post, put, patch) = (Verb::Post, Verb::Put, Verb::Patch);
    assert_eq!(
        verbs,
        [post, post, patch, put, post, post, post, post, post]
    );
    for change in &changes {
        assert_eq!(
            render(change).verb(),
            change.verb(),
            "{}",
            change.function()
        );
    }
}

#[test]
fn prj_028_each_change_names_its_function_and_repository_for_the_audit() {
    let changes = every_change(
        &repository(),
        Number::new(12).unwrap(),
        RunId::parse("1").unwrap(),
    );
    let mut functions: Vec<&str> = changes.iter().map(Change::function).collect();
    assert_eq!(functions[3], "merge_pull_request");
    functions.sort_unstable();
    functions.dedup();
    assert_eq!(functions.len(), 9, "one name for each change");
    assert!(
        changes
            .iter()
            .all(|change| change.repository() == &repository())
    );
}

#[test]
fn od_e14_a_rendered_path_is_its_parts_below_the_repository() {
    let change = &every_change(
        &repository(),
        Number::new(12).unwrap(),
        RunId::parse("7").unwrap(),
    )[3];
    let rendered = render(change);
    assert_eq!(
        rendered.path(),
        "/repos/octo-org/hello.world/pulls/12/merge"
    );
    assert_eq!(rendered.body(), Some(&json!({})));
    assert!(Request::get(rendered.path(), &[]).is_ok());
}

#[test]
#[should_panic(expected = "a fixed segment")]
fn od_e14_a_fixed_segment_cannot_climb() {
    let _ = Fixed::new("..");
}

#[test]
#[should_panic(expected = "a fixed segment")]
fn od_e14_a_fixed_segment_cannot_hold_a_slash() {
    let _ = Fixed::new("app/installations");
}

#[test]
fn src_001_a_source_that_makes_changes_declares_both_functions() {
    let changes = Changes { render, settle };
    let change = &every_change(
        &repository(),
        Number::new(1).unwrap(),
        RunId::parse("1").unwrap(),
    )[0];
    assert_eq!(
        (changes.render)(change).path(),
        "/repos/octo-org/hello.world/issues"
    );
    let reply = ChangeReply {
        status: 201,
        body: crate::Body::Read(b"{}"),
        limit: crate::Limit::default(),
        at: chrono::DateTime::UNIX_EPOCH,
    };
    assert_eq!(
        (changes.settle)(change, &reply),
        Settled::Done(Receipt::Made)
    );
}

fn hostile() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<String>(),
        "[a-zA-Z0-9./?#%@:_ \\\\\\x00-\\x1f-]{1,16}"
    ]
}

proptest! {
    #[test]
    fn od_e14_a_rendered_path_never_leaves_the_repository(
        owner in hostile(), name in hostile(), number in any::<u64>(), run in hostile(),
    ) {
        let repository = RepositoryName::parse(&format!("{owner}/{name}"))
            .unwrap_or_else(|_| repository());
        let number = Number::new(number).unwrap_or_else(|_| Number::new(1).unwrap());
        let run = RunId::parse(&run).unwrap_or_else(|_| RunId::parse("1").unwrap());
        let prefix = format!("/repos/{}/{}/", repository.owner(), repository.name());
        for change in every_change(&repository, number, run) {
            let rendered = render(&change);
            prop_assert!(rendered.path().starts_with(&prefix));
            prop_assert!(Request::get(rendered.path(), &[]).is_ok(), "{}", rendered.path());
            let plain = rendered.path().bytes().all(|b| b.is_ascii_alphanumeric() || b"/.-_".contains(&b));
            prop_assert!(plain, "{}", rendered.path());
        }
    }
}
