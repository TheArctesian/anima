//! Each change is the verb, path and JSON body GitHub documents for it (OD-E14).

use serde_json::{Value, json};
use source_exchange::{
    BranchName, Change, CloseReason, InputName, InputValue, Inputs, IssueState, Label, Labels,
    MergeMethod, StatusContext, StatusDescription, StatusState, TargetUrl, Text, Title, Verb,
    WorkflowId,
};

use super::{SHA, every_change, number, repository, sha};
use crate::changes::render;

fn assert_rendered(change: &Change, verb: Verb, path: &str, body: Option<&Value>) {
    let rendered = render(change);
    assert_eq!(rendered.verb(), verb, "{}", change.function());
    assert_eq!(rendered.path(), path, "{}", change.function());
    assert_eq!(rendered.body(), body, "{}", change.function());
}

#[test]
fn prj_031_create_issue_posts_the_title_body_and_labels() {
    let change = Change::CreateIssue {
        repository: repository(),
        title: Title::parse("Found a bug").unwrap(),
        body: Some(Text::parse("It breaks on\nlaunch.").unwrap()),
        labels: Labels::new(vec![
            Label::parse("bug").unwrap(),
            Label::parse("good first issue").unwrap(),
        ])
        .unwrap(),
    };
    assert_rendered(
        &change,
        Verb::Post,
        "/repos/octo-org/hello.world/issues",
        Some(&json!({
            "title": "Found a bug",
            "body": "It breaks on\nlaunch.",
            "labels": ["bug", "good first issue"],
        })),
    );
}

#[test]
fn prj_031_create_issue_leaves_out_an_absent_body_and_no_labels() {
    assert_rendered(
        &every_change()[0],
        Verb::Post,
        "/repos/octo-org/hello.world/issues",
        Some(&json!({ "title": "Found a bug" })),
    );
}

#[test]
fn prj_031_a_comment_posts_its_body_below_the_issue() {
    assert_rendered(
        &every_change()[1],
        Verb::Post,
        "/repos/octo-org/hello.world/issues/1347/comments",
        Some(&json!({ "body": "Me too" })),
    );
}

#[test]
fn prj_031_set_issue_open_patches_the_state_and_the_reason() {
    let path = "/repos/octo-org/hello.world/issues/1347";
    let cases = [
        (IssueState::Open, json!({ "state": "open" })),
        (IssueState::Closed(None), json!({ "state": "closed" })),
        (
            IssueState::Closed(Some(CloseReason::Completed)),
            json!({ "state": "closed", "state_reason": "completed" }),
        ),
        (
            IssueState::Closed(Some(CloseReason::NotPlanned)),
            json!({ "state": "closed", "state_reason": "not_planned" }),
        ),
    ];
    for (state, body) in cases {
        let change = Change::SetIssueOpen {
            repository: repository(),
            number: number(),
            state,
        };
        assert_rendered(&change, Verb::Patch, path, Some(&body));
    }
}

#[test]
fn prj_033_merge_puts_the_method_and_the_head_it_must_still_have() {
    let methods = [
        (MergeMethod::Merge, "merge"),
        (MergeMethod::Squash, "squash"),
        (MergeMethod::Rebase, "rebase"),
    ];
    for (method, word) in methods {
        let change = Change::MergePullRequest {
            repository: repository(),
            number: number(),
            head: sha(),
            method,
        };
        assert_rendered(
            &change,
            Verb::Put,
            "/repos/octo-org/hello.world/pulls/1347/merge",
            Some(&json!({ "merge_method": word, "sha": SHA })),
        );
    }
}

#[test]
fn prj_034_the_run_controls_post_to_the_run_with_no_body() {
    let changes = every_change();
    let base = "/repos/octo-org/hello.world/actions/runs/30433642";
    assert_rendered(&changes[4], Verb::Post, &format!("{base}/rerun"), None);
    assert_rendered(
        &changes[5],
        Verb::Post,
        &format!("{base}/rerun-failed-jobs"),
        None,
    );
    assert_rendered(&changes[6], Verb::Post, &format!("{base}/cancel"), None);
}

#[test]
fn prj_035_dispatch_posts_the_branch_and_the_inputs_as_strings_in_the_body() {
    let inputs = Inputs::new(vec![
        (
            InputName::parse("environment").unwrap(),
            InputValue::parse("staging").unwrap(),
        ),
        (
            InputName::parse("dry-run").unwrap(),
            InputValue::parse("true").unwrap(),
        ),
    ])
    .unwrap();
    let change = Change::DispatchWorkflow {
        repository: repository(),
        workflow: WorkflowId::parse("release.yaml").unwrap(),
        branch: BranchName::parse("release/v1.2").unwrap(),
        inputs,
    };
    assert_rendered(
        &change,
        Verb::Post,
        "/repos/octo-org/hello.world/actions/workflows/release.yaml/dispatches",
        Some(&json!({
            "ref": "release/v1.2",
            "inputs": { "environment": "staging", "dry-run": "true" },
        })),
    );
}

#[test]
fn prj_035_dispatch_by_workflow_number_with_no_inputs_sends_an_empty_object() {
    let change = Change::DispatchWorkflow {
        repository: repository(),
        workflow: WorkflowId::parse("161335").unwrap(),
        branch: BranchName::parse("main").unwrap(),
        inputs: Inputs::default(),
    };
    assert_rendered(
        &change,
        Verb::Post,
        "/repos/octo-org/hello.world/actions/workflows/161335/dispatches",
        Some(&json!({ "ref": "main", "inputs": {} })),
    );
}

#[test]
fn pro_010_a_commit_status_posts_the_state_context_description_and_address() {
    let change = Change::SetCommitStatus {
        repository: repository(),
        sha: sha(),
        state: StatusState::Failure,
        context: StatusContext::parse("anima/build").unwrap(),
        description: Some(StatusDescription::parse("2 of 5 targets failed").unwrap()),
        target_url: Some(
            TargetUrl::parse("https://hub.example/builds/7", "https://hub.example").unwrap(),
        ),
    };
    assert_rendered(
        &change,
        Verb::Post,
        &format!("/repos/octo-org/hello.world/statuses/{SHA}"),
        Some(&json!({
            "state": "failure",
            "context": "anima/build",
            "description": "2 of 5 targets failed",
            "target_url": "https://hub.example/builds/7",
        })),
    );
}

#[test]
fn pro_010_a_commit_status_leaves_out_what_is_not_set() {
    let expected = [
        (StatusState::Pending, "pending"),
        (StatusState::Success, "success"),
        (StatusState::Error, "error"),
    ];
    for (state, word) in expected {
        let Change::SetCommitStatus {
            repository,
            sha,
            context,
            ..
        } = every_change().swap_remove(8)
        else {
            panic!("the ninth change is a commit status");
        };
        let change = Change::SetCommitStatus {
            repository,
            sha,
            state,
            context,
            description: None,
            target_url: None,
        };
        assert_rendered(
            &change,
            Verb::Post,
            &format!("/repos/octo-org/hello.world/statuses/{SHA}"),
            Some(&json!({ "state": word, "context": "anima/build" })),
        );
    }
}
