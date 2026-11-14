//! Hostile-looking but valid parts never move a change off its repository's endpoints, and
//! text goes only into the body (OD-E14, design F leak tests).

use proptest::prelude::*;
use regex::Regex;
use source_exchange::{
    BranchName, Change, CommitSha, InputName, InputValue, Inputs, IssueState, Label, Labels,
    MergeMethod, Number, RepositoryName, Request, RunId, StatusContext, StatusDescription,
    StatusState, TargetUrl, Text, Title, Verb, WorkflowId,
};

use crate::changes::render;

const ENDPOINTS: &str =
    r"^/repos/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/(issues|pulls|actions|statuses)(/|$)";

/// The verb GitHub documents for each function.
fn documented_verb(function: &str) -> Verb {
    match function {
        "set_issue_open" => Verb::Patch,
        "merge_pull_request" => Verb::Put,
        _ => Verb::Post,
    }
}

/// The parts of one change of each kind.
#[derive(Debug)]
struct Parts {
    repository: RepositoryName,
    number: Number,
    run: RunId,
    sha: CommitSha,
    workflow: WorkflowId,
    branch: BranchName,
    title: Title,
    text: Text,
    label: Label,
    input: (InputName, InputValue),
    description: StatusDescription,
    target_url: TargetUrl,
}

fn part<T: std::fmt::Debug>(
    pattern: &'static str,
    parse: fn(&str) -> Option<T>,
) -> impl Strategy<Value = T> {
    pattern.prop_filter_map("a valid part", move |text| parse(&text))
}

fn repository() -> impl Strategy<Value = RepositoryName> {
    ("[A-Za-z0-9._-]{1,39}", "[A-Za-z0-9._-]{1,100}")
        .prop_filter_map("a valid repository", |(owner, name)| {
            RepositoryName::parse(&format!("{owner}/{name}")).ok()
        })
}

fn workflow() -> impl Strategy<Value = WorkflowId> {
    prop_oneof![
        part("[1-9][0-9]{0,14}", |t| WorkflowId::parse(t).ok()),
        part("[A-Za-z0-9._-]{1,60}[.]ya?ml", |t| WorkflowId::parse(t)
            .ok()),
    ]
}

fn parts() -> impl Strategy<Value = Parts> {
    let names = (
        repository(),
        1..=(1_u64 << 53),
        1..=(1_u64 << 53),
        part("[0-9a-fA-F]{40}", |t| CommitSha::parse(t).ok()),
        workflow(),
        part("[A-Za-z0-9/#?%@&=+._~-]{1,40}|\\PC{1,20}", |t| {
            BranchName::parse(t).ok()
        }),
    );
    let text = (
        part("\\PC{1,40}", |t| Title::parse(t).ok()),
        part("[\\PC\n\t\r]{1,80}", |t| Text::parse(t).ok()),
        part("\\PC{1,20}", |t| Label::parse(t).ok()),
        part("[A-Za-z0-9_-]{1,20}", |t| InputName::parse(t).ok()),
        part("[\\PC\n]{0,40}", |t| InputValue::parse(t).ok()),
        part("\\PC{0,40}", |t| StatusDescription::parse(t).ok()),
        part("/[^ \\\\]{0,40}", |t| {
            TargetUrl::parse(&format!("https://hub.example{t}"), "https://hub.example").ok()
        }),
    );
    (names, text).prop_map(|((repository, number, run, sha, workflow, branch), text)| {
        let (title, text, label, name, value, description, target_url) = text;
        Parts {
            repository,
            number: Number::new(number).unwrap(),
            run: RunId::parse(&run.to_string()).unwrap(),
            sha,
            workflow,
            branch,
            title,
            text,
            label,
            input: (name, value),
            description,
            target_url,
        }
    })
}

/// One change of each kind, every optional part set.
fn every_change(p: &Parts) -> Vec<Change> {
    let repository = || p.repository.clone();
    vec![
        Change::CreateIssue {
            repository: repository(),
            title: p.title.clone(),
            body: Some(p.text.clone()),
            labels: Labels::new(vec![p.label.clone()]).unwrap(),
        },
        Change::AddIssueComment {
            repository: repository(),
            number: p.number,
            body: p.text.clone(),
        },
        Change::SetIssueOpen {
            repository: repository(),
            number: p.number,
            state: IssueState::Closed(None),
        },
        Change::MergePullRequest {
            repository: repository(),
            number: p.number,
            head: p.sha.clone(),
            method: MergeMethod::Rebase,
        },
        Change::RerunWorkflowRun {
            repository: repository(),
            run: p.run,
        },
        Change::RerunFailedWorkflowJobs {
            repository: repository(),
            run: p.run,
        },
        Change::CancelWorkflowRun {
            repository: repository(),
            run: p.run,
        },
        Change::DispatchWorkflow {
            repository: repository(),
            workflow: p.workflow.clone(),
            branch: p.branch.clone(),
            inputs: Inputs::new(vec![p.input.clone()]).unwrap(),
        },
        Change::SetCommitStatus {
            repository: repository(),
            sha: p.sha.clone(),
            state: StatusState::Pending,
            context: StatusContext::parse("anima/build").unwrap(),
            description: Some(p.description.clone()),
            target_url: Some(p.target_url.clone()),
        },
    ]
}

proptest! {
    #[test]
    fn od_e14_every_rendered_path_stays_on_the_repository_endpoints(p in parts()) {
        let endpoints = Regex::new(ENDPOINTS).unwrap();
        for change in every_change(&p) {
            let rendered = render(&change);
            let path = rendered.path();
            prop_assert!(endpoints.is_match(path), "{path}");
            let prefix = format!("/repos/{}/", p.repository);
            prop_assert!(path.starts_with(&prefix), "{path}");
            let plain = path.bytes().all(|b| b.is_ascii_alphanumeric() || b"/.-_".contains(&b));
            prop_assert!(plain, "{path}");
            prop_assert!(Request::get(path, &[]).is_ok(), "{path}");
            prop_assert_eq!(rendered.verb(), change.verb());
            prop_assert_eq!(rendered.verb(), documented_verb(change.function()));
        }
    }

    #[test]
    fn od_e14_text_and_the_branch_go_into_the_body_unchanged(p in parts()) {
        let changes = every_change(&p);
        let body = |at: usize| render(&changes[at]).body().cloned().unwrap();
        prop_assert_eq!(&body(0)["title"], p.title.as_str());
        prop_assert_eq!(&body(0)["body"], p.text.as_str());
        prop_assert_eq!(&body(0)["labels"][0], p.label.as_str());
        prop_assert_eq!(&body(1)["body"], p.text.as_str());
        let dispatch = body(7);
        prop_assert_eq!(&dispatch["ref"], p.branch.as_str());
        prop_assert_eq!(&dispatch["inputs"][p.input.0.as_str()], p.input.1.as_str());
        prop_assert_eq!(&body(8)["description"], p.description.as_str());
        prop_assert_eq!(&body(8)["target_url"], p.target_url.as_str());
    }
}
