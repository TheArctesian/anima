//! PRJ-031: issues and the comments of an issue or pull request.

use super::{fixture, ok, path_and_query};
use crate::live::StateFilter;
use crate::live::comments;
use crate::live::issues::{CloseReason, list_parse, list_request, one_parse, one_request};

const REPOSITORY: &str = "sample-org/sample-repo";

#[test]
fn prj_031_the_issue_requests_have_the_documented_paths_and_queries() {
    let list = list_request(REPOSITORY, StateFilter::Closed, 2).unwrap();
    assert_eq!(
        path_and_query(&list),
        "/repos/sample-org/sample-repo/issues?state=closed&sort=updated&direction=desc&per_page=100&page=2"
    );
    let one = one_request(REPOSITORY, 21).unwrap();
    assert_eq!(
        path_and_query(&one),
        "/repos/sample-org/sample-repo/issues/21"
    );
    let thread = comments::request(REPOSITORY, 21, 1).unwrap();
    assert_eq!(
        path_and_query(&thread),
        "/repos/sample-org/sample-repo/issues/21/comments?per_page=100&page=1"
    );
}

#[test]
fn prj_031_an_issue_list_drops_the_entries_that_are_pull_requests() {
    let body = fixture("live_issues_list.json");

    let issues = list_parse(&ok(&body)).unwrap();

    let numbers: Vec<u64> = issues.iter().map(|issue| issue.number).collect();
    assert_eq!(numbers, [21, 5, 4], "number 12 is a pull request");
}

#[test]
fn prj_031_the_issue_fixture_parses_to_its_values() {
    let body = fixture("live_issues_list.json");

    let issues = list_parse(&ok(&body)).unwrap();

    let open = &issues[0];
    assert_eq!(open.title, "The sample page is slow");
    assert!(open.open);
    assert_eq!(open.close_reason, None);
    assert_eq!(open.author_login, "octo-reporter");
    assert_eq!(open.labels, ["bug", "perf"]);
    assert_eq!(open.assignees, ["octo-writer"]);
    assert_eq!(open.comments, 2);
    assert_eq!(open.closed_at, None);
    assert_eq!(open.body.as_deref(), Some("It takes ten seconds."));
    let closed = &issues[1];
    assert!(!closed.open);
    assert!(closed.closed_at.is_some());
    assert!(closed.assignees.is_empty(), "null assignees are none");
}

#[test]
fn prj_031_state_reason_maps_documented_names_and_everything_else_to_other() {
    let issues = list_parse(&ok(&fixture("live_issues_list.json"))).unwrap();
    assert_eq!(issues[1].close_reason, Some(CloseReason::NotPlanned));
    assert_eq!(issues[2].close_reason, Some(CloseReason::Other));

    for (name, expected) in [
        ("completed", CloseReason::Completed),
        ("not_planned", CloseReason::NotPlanned),
        ("reopened", CloseReason::Reopened),
        ("duplicate", CloseReason::Duplicate),
    ] {
        let mut issue: serde_json::Value =
            serde_json::from_slice(&fixture("live_issues_list.json")).unwrap();
        let mut one = issue[2].take();
        one["state_reason"] = serde_json::json!(name);
        let body = serde_json::to_vec(&one).unwrap();
        assert_eq!(
            one_parse(&ok(&body)).unwrap().close_reason,
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn prj_031_the_comments_fixture_parses_in_order() {
    let body = fixture("live_issue_comments.json");

    let thread = comments::parse(&ok(&body)).unwrap();

    assert_eq!(thread.len(), 2);
    assert_eq!(thread[0].id, 9001);
    assert_eq!(thread[0].author_login, "octo-writer");
    assert_eq!(thread[0].body, "First reply.");
    assert_eq!(
        thread[0].url,
        "https://github.com/sample-org/sample-repo/issues/21#issuecomment-9001"
    );
    assert_eq!(thread[1].author_login, "ghost");
    assert_eq!(thread[1].body, "");
}
