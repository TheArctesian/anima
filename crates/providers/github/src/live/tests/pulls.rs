//! PRJ-031 and PRJ-033: pull requests, their mergeability and the repository's merge methods.

use super::{fixture, ok, path_and_query};
use crate::live::StateFilter;
use crate::live::pulls::{
    MergeMethod, MergeableState, PullState, list_parse, list_request, merge_methods_parse,
    merge_methods_request, one_parse, one_request,
};

const REPOSITORY: &str = "sample-org/sample-repo";

#[test]
fn prj_031_the_pull_request_list_has_the_documented_path_and_query() {
    let request = list_request(REPOSITORY, StateFilter::Open, 3).unwrap();

    assert_eq!(
        path_and_query(&request),
        "/repos/sample-org/sample-repo/pulls?state=open&sort=created&direction=desc&per_page=100&page=3"
    );
    let all = list_request(REPOSITORY, StateFilter::All, 1).unwrap();
    assert!(path_and_query(&all).contains("state=all"));
    let closed = list_request(REPOSITORY, StateFilter::Closed, 0).unwrap();
    assert!(
        path_and_query(&closed)
            .ends_with("state=closed&sort=created&direction=desc&per_page=100&page=1")
    );
}

#[test]
fn prj_031_the_pull_request_list_fixture_parses_to_its_values() {
    let body = fixture("live_pulls_list.json");

    let pulls = list_parse(&ok(&body)).unwrap();

    assert_eq!(pulls.len(), 3);
    let open = &pulls[0];
    assert_eq!(open.number, 12);
    assert_eq!(open.title, "Add the sample widget");
    assert_eq!(open.state, PullState::Open);
    assert!(!open.draft);
    assert_eq!(open.author_login, "octo-writer");
    assert_eq!(open.head_branch, "feature/widget");
    assert_eq!(open.head_sha, "1".repeat(40));
    assert_eq!(open.base_branch, "main");
}

#[test]
fn prj_031_a_list_entry_has_its_fork_flag_labels_dates_and_page() {
    let body = fixture("live_pulls_list.json");

    let pulls = list_parse(&ok(&body)).unwrap();

    let open = &pulls[0];
    assert!(!open.from_fork);
    assert_eq!(
        open.head_repository.as_deref(),
        Some("sample-org/sample-repo")
    );
    assert_eq!(open.labels, ["enhancement", "ui"]);
    assert_eq!(open.comments, None);
    assert_eq!(open.merged_at, None);
    assert_eq!(
        open.url,
        "https://github.com/sample-org/sample-repo/pull/12"
    );
    assert_eq!(
        open.body.as_deref(),
        Some("Adds the widget.\n\n<b>not html here</b>")
    );
}

#[test]
fn prj_031_a_closed_pull_request_with_a_merge_time_is_merged_and_one_from_a_fork_says_so() {
    let body = fixture("live_pulls_list.json");

    let pulls = list_parse(&ok(&body)).unwrap();

    let merged = &pulls[1];
    assert_eq!(merged.state, PullState::Merged);
    assert!(merged.merged_at.is_some());
    assert!(merged.draft);
    assert!(merged.from_fork);
    assert_eq!(
        merged.head_repository.as_deref(),
        Some("someone-else/sample-repo")
    );
    assert_eq!(
        merged.author_login, "ghost",
        "a deleted account has no user"
    );
    assert_eq!(merged.body, None);
}

#[test]
fn prj_033_a_pull_request_whose_head_repository_was_deleted_counts_as_from_a_fork() {
    let body = fixture("live_pulls_list.json");

    let pulls = list_parse(&ok(&body)).unwrap();

    let orphan = &pulls[2];
    assert_eq!(orphan.state, PullState::Closed);
    assert!(orphan.from_fork);
    assert_eq!(orphan.head_repository, None);
    assert!(!orphan.draft, "an absent draft flag is not a draft");
}

#[test]
fn prj_033_the_single_pull_request_request_and_fixture() {
    let request = one_request(REPOSITORY, 12).unwrap();
    assert_eq!(
        path_and_query(&request),
        "/repos/sample-org/sample-repo/pulls/12"
    );

    let body = fixture("live_pull_one.json");
    let detail = one_parse(&ok(&body)).unwrap();

    assert_eq!(detail.pull.number, 12);
    assert_eq!(detail.pull.comments, Some(3));
    assert_eq!(detail.mergeable, Some(true));
    assert_eq!(detail.mergeable_state, MergeableState::Clean);
}

fn state_of(mergeable_state: &serde_json::Value) -> MergeableState {
    let mut pull: serde_json::Value =
        serde_json::from_slice(&fixture("live_pull_one.json")).unwrap();
    pull["mergeable_state"] = mergeable_state.clone();
    let body = serde_json::to_vec(&pull).unwrap();
    one_parse(&ok(&body)).unwrap().mergeable_state
}

#[test]
fn prj_033_each_documented_mergeable_state_maps_to_its_variant() {
    for (name, expected) in [
        ("clean", MergeableState::Clean),
        ("unstable", MergeableState::Unstable),
        ("dirty", MergeableState::Dirty),
        ("blocked", MergeableState::Blocked),
        ("behind", MergeableState::Behind),
        ("draft", MergeableState::Draft),
        ("has_hooks", MergeableState::HasHooks),
        ("unknown", MergeableState::Unknown),
        ("CLEAN", MergeableState::Clean),
        ("Behind", MergeableState::Behind),
    ] {
        assert_eq!(state_of(&serde_json::json!(name)), expected, "{name}");
    }
}

#[test]
fn prj_033_an_unknown_or_null_mergeable_state_is_unknown_never_clean() {
    assert_eq!(
        state_of(&serde_json::json!("brand_new_state")),
        MergeableState::Unknown
    );
    assert_eq!(state_of(&serde_json::json!("")), MergeableState::Unknown);
    assert_eq!(state_of(&serde_json::Value::Null), MergeableState::Unknown);
}

#[test]
fn prj_033_mergeable_is_none_while_github_computes_it() {
    let mut pull: serde_json::Value =
        serde_json::from_slice(&fixture("live_pull_one.json")).unwrap();
    pull["mergeable"] = serde_json::Value::Null;
    let body = serde_json::to_vec(&pull).unwrap();

    assert_eq!(one_parse(&ok(&body)).unwrap().mergeable, None);
}

#[test]
fn prj_033_the_merge_methods_are_read_from_the_repository() {
    let request = merge_methods_request(REPOSITORY).unwrap();
    assert_eq!(path_and_query(&request), "/repos/sample-org/sample-repo");

    let body = fixture("live_repository_merge_methods.json");

    assert_eq!(
        merge_methods_parse(&ok(&body)).unwrap(),
        [MergeMethod::Squash, MergeMethod::Rebase]
    );
}

#[test]
fn prj_033_absent_merge_method_fields_mean_no_method() {
    let without = br#"{"id": 500, "full_name": "sample-org/sample-repo"}"#;
    assert_eq!(merge_methods_parse(&ok(without)).unwrap(), []);

    let partly = br#"{"id": 500, "allow_merge_commit": true, "allow_squash_merge": null}"#;
    assert_eq!(
        merge_methods_parse(&ok(partly)).unwrap(),
        [MergeMethod::Merge]
    );
}
