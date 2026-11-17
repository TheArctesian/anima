//! `anima snapshot list` and `delete`, and the errors of every `snapshot` command (JOB-018,
//! CMD-021, CMD-024).

use super::repos::Repos;

/// A repository with one pushed snapshot; its ref.
async fn pushed() -> (Repos, String, String) {
    let repos = Repos::new();
    repos.write("a.txt", "changed\n");
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let pushed = run.json();
    let scratch = pushed["ref"].as_str().expect("a ref").to_owned();
    let commit = pushed["commit"].as_str().expect("a commit").to_owned();
    (repos, scratch, commit)
}

#[tokio::test]
async fn job_018_list_shows_each_scratch_ref_and_its_commit() {
    let (repos, scratch, commit) = pushed().await;
    let run = repos.anima(&["snapshot", "list", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.json(),
        serde_json::json!([{ "ref": scratch, "commit": commit }])
    );
    let text = repos.anima(&["snapshot", "list"]).await;
    assert!(
        text.stdout.contains("REF") && text.stdout.contains(&scratch),
        "{text:?}"
    );
    assert!(!text.stdout.contains("refs/heads/main"), "{text:?}");
}

#[tokio::test]
async fn job_018_delete_removes_the_one_scratch_ref() {
    let (repos, scratch, commit) = pushed().await;
    let id = scratch.trim_start_matches("refs/anima/wip/");
    let main = repos.git_in(&repos.remote(), &["rev-parse", "main"]);

    let run = repos
        .anima(&["snapshot", "delete", id, "--yes", "--json"])
        .await;

    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["ref"], scratch.as_str());
    assert_eq!(run.json()["commit"], commit.as_str());
    assert_eq!(repos.remote_scratch_refs(), "");
    assert_eq!(repos.git_in(&repos.remote(), &["rev-parse", "main"]), main);
    let list = repos.anima(&["snapshot", "list", "--json"]).await;
    assert_eq!(list.json(), serde_json::json!([]));
}

#[tokio::test]
async fn cmd_024_delete_without_yes_and_no_terminal_asks_for_yes_and_keeps_the_ref() {
    let (repos, scratch, _) = pushed().await;
    let run = repos.anima(&["snapshot", "delete", &scratch]).await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--yes"), "{run:?}");
    assert!(repos.remote_scratch_refs().contains(&scratch));
}

#[tokio::test]
async fn job_018_delete_refuses_a_ref_outside_the_scratch_space() {
    let (repos, scratch, _) = pushed().await;
    let main = repos.git_in(&repos.remote(), &["rev-parse", "main"]);
    for target in [
        "refs/heads/main",
        "refs/tags/v1",
        "refs/anima/wip/../heads/main",
    ] {
        let run = repos
            .anima(&["snapshot", "delete", target, "--yes", "--json"])
            .await;
        assert_eq!(run.code, 2, "{target}: {run:?}");
        assert_eq!(run.error()["code"], "USAGE", "{target}");
    }
    assert_eq!(repos.git_in(&repos.remote(), &["rev-parse", "main"]), main);
    assert!(repos.remote_scratch_refs().contains(&scratch));
}

#[tokio::test]
async fn job_018_deleting_a_scratch_ref_that_is_not_there_is_not_found() {
    let (repos, _, _) = pushed().await;
    let run = repos
        .anima(&["snapshot", "delete", "0000000000000000", "--yes"])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn cmd_021_outside_a_git_working_tree_is_a_usage_error() {
    let repos = Repos::new();
    for args in [
        ["snapshot", "push", "--json"],
        ["snapshot", "list", "--json"],
    ] {
        let run = repos.anima_in(&repos.plain_folder(), &args).await;
        assert_eq!(run.code, 2, "{run:?}");
        assert_eq!(run.error()["code"], "USAGE");
        assert!(
            run.error()["message"]
                .as_str()
                .unwrap()
                .contains("not inside a git working tree")
        );
    }
}

#[tokio::test]
async fn cmd_021_an_unknown_remote_is_not_found_and_a_dash_remote_is_usage() {
    let repos = Repos::new();
    repos.write("a.txt", "changed\n");
    let before = repos.state();
    let run = repos
        .anima(&["snapshot", "push", "--remote", "nope", "--json"])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(run.error()["code"], "NOT_FOUND");
    assert!(run.error()["message"].as_str().unwrap().contains("nope"));

    let run = repos
        .anima(&["snapshot", "push", "--remote=--receive-pack=x", "--json"])
        .await;
    assert_eq!(run.code, 2, "{run:?}");

    let run = repos.anima(&["snapshot", "list", "--remote", "nope"]).await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(repos.state(), before);
    assert_eq!(repos.remote_scratch_refs(), "");
}

#[tokio::test]
async fn cmd_021_a_remote_that_cannot_be_reached_is_unavailable() {
    let repos = Repos::new();
    repos.git(&["remote", "add", "gone", "/nonexistent/anima/remote.git"]);
    let run = repos
        .anima(&["snapshot", "push", "--remote", "gone", "--json"])
        .await;
    assert_eq!(run.code, 7, "{run:?}");
    assert_eq!(run.error()["code"], "UNAVAILABLE");
}
