//! PRJ-031: the commits of a branch, the branches and the releases of a repository.

use super::{fixture, ok, path_and_query};
use crate::live::{commits, refs};

const REPOSITORY: &str = "sample-org/sample-repo";

#[test]
fn prj_031_the_history_requests_have_the_documented_paths_and_queries() {
    let default = commits::list_request(REPOSITORY, None, 1).unwrap();
    assert_eq!(
        path_and_query(&default),
        "/repos/sample-org/sample-repo/commits?per_page=100&page=1"
    );
    let branch = commits::list_request(REPOSITORY, Some("feature/widget"), 3).unwrap();
    assert_eq!(
        path_and_query(&branch),
        "/repos/sample-org/sample-repo/commits?sha=feature/widget&per_page=100&page=3"
    );
    let sha = "ab".repeat(20);
    let pinned = commits::list_request(REPOSITORY, Some(&sha), 0).unwrap();
    assert_eq!(
        path_and_query(&pinned),
        format!("/repos/sample-org/sample-repo/commits?sha={sha}&per_page=100&page=1")
    );
    assert_eq!(
        path_and_query(&refs::branches_request(REPOSITORY, 2).unwrap()),
        "/repos/sample-org/sample-repo/branches?per_page=100&page=2"
    );
    assert_eq!(
        path_and_query(&refs::releases_request(REPOSITORY, 1).unwrap()),
        "/repos/sample-org/sample-repo/releases?per_page=100&page=1"
    );
}

#[test]
fn prj_031_the_commit_list_fixture_parses_to_its_values() {
    let body = fixture("live_commits_list.json");

    let list = commits::list_parse(&ok(&body)).unwrap();

    assert_eq!(list.len(), 3);
    let first = &list[0];
    assert_eq!(first.sha, "1".repeat(40));
    assert_eq!(first.title, "Add the widget");
    assert_eq!(
        first.message,
        "Add the widget\n\nIt draws a box.\nSecond body line."
    );
    assert_eq!(first.author_name, "Ada Sample");
    assert_eq!(first.author_login.as_deref(), Some("octo-writer"));
    assert_eq!(first.authored_at.to_rfc3339(), "2026-11-02T09:00:00+00:00");
    assert_eq!(first.committed_at.to_rfc3339(), "2026-11-02T09:30:00+00:00");
    assert_eq!(
        first.url,
        format!(
            "https://github.com/sample-org/sample-repo/commit/{}",
            "1".repeat(40)
        )
    );
    assert_eq!(first.parents, 1);
}

#[test]
fn prj_031_a_commit_without_an_account_or_a_committer_still_reads() {
    let body = fixture("live_commits_list.json");

    let list = commits::list_parse(&ok(&body)).unwrap();

    let merge = &list[1];
    assert_eq!(merge.author_login, None, "an empty account object");
    assert_eq!(merge.author_name, "Bo Sample");
    assert_eq!(merge.parents, 2);
    let root = &list[2];
    assert_eq!(root.parents, 0);
    assert_eq!(root.title, "Initial commit");
    assert_eq!(root.committed_at, root.authored_at, "no committer");
}

#[test]
fn prj_031_the_branch_list_fixture_parses_to_its_values() {
    let body = fixture("live_branches_list.json");

    let list = refs::branches_parse(&ok(&body)).unwrap();

    let named: Vec<_> = list
        .iter()
        .map(|b| (b.name.as_str(), b.head_sha.as_str(), b.protected))
        .collect();
    assert_eq!(
        named,
        [
            ("main", "4".repeat(40).as_str(), true),
            ("feature/widget", "5".repeat(40).as_str(), false),
            ("old", "6".repeat(40).as_str(), false),
        ]
    );
}

#[test]
fn prj_031_the_release_list_fixture_parses_to_its_values() {
    let body = fixture("live_releases_list.json");

    let list = refs::releases_parse(&ok(&body)).unwrap();

    assert_eq!(list.len(), 2);
    let released = &list[0];
    assert_eq!(released.id, 801);
    assert_eq!(released.tag, "v1.2.0");
    assert_eq!(released.name.as_deref(), Some("Version 1.2"));
    assert!(!released.draft && !released.prerelease);
    assert_eq!(
        released.published_at.unwrap().to_rfc3339(),
        "2026-10-30T12:00:00+00:00"
    );
    assert_eq!(released.author_login, "octo-writer");
    assert_eq!(
        released.url,
        "https://github.com/sample-org/sample-repo/releases/tag/v1.2.0"
    );
    assert_eq!(released.body, "Notes for 1.2.");
    let draft = &list[1];
    assert!(draft.draft && draft.prerelease);
    assert_eq!(draft.name, None);
    assert_eq!(draft.published_at, None);
    assert_eq!(draft.author_login, "ghost");
    assert_eq!(draft.body, "");
}

#[test]
fn prj_031_a_history_list_that_is_not_ok_or_not_a_list_is_a_problem() {
    let wrong = ok(b"{\"unexpected\": true}");
    assert_eq!(
        commits::list_parse(&wrong).unwrap_err().code,
        "reply_unreadable"
    );
    assert_eq!(
        refs::branches_parse(&wrong).unwrap_err().code,
        "reply_unreadable"
    );
    assert_eq!(
        refs::releases_parse(&wrong).unwrap_err().code,
        "reply_unreadable"
    );
    let missing = source_exchange::Response {
        link: None,
        status: 404,
        body: source_exchange::Body::Read(b"{}"),
    };
    assert_eq!(
        commits::list_parse(&missing).unwrap_err().code,
        "unexpected_status"
    );
}
