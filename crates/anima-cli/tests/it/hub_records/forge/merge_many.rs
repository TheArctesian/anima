//! `anima pr merge` with several numbers against the scripted hub (PRJ-033, PRO-013, CMD-021):
//! it reads them all, asks once, prints one outcome for each and never takes a fork.

use serde_json::json;

use super::merge::{OTHER_SHA, merges, read_as};
use super::{ask, variables};
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::forge::{HUB_CODES, REPOSITORY, SHA, fork_pull_request, pull_request};

#[tokio::test]
async fn prj_033_merging_several_confirms_once_and_exits_1_when_one_is_not_merged() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "First", None));
    let mut second = pull_request(8, "Second", None);
    second["headSha"] = json!(OTHER_SHA);
    read_as(&hub, &second);
    let outcomes = json!([
        { "repository": REPOSITORY, "number": 7, "outcome": "MERGED", "message": "Merged.",
          "mergeSha": OTHER_SHA },
        { "repository": REPOSITORY, "number": 8, "outcome": "NOT_MERGEABLE",
          "message": "It cannot merge now.", "mergeSha": null },
    ]);
    hub.answer(
        "PullRequestMergeMany",
        data(json!({ "mergePullRequests": outcomes })),
    );
    let run = ask(
        &hub,
        &[
            "pr", "merge", REPOSITORY, "7", "8", "--method", "rebase", "--yes",
        ],
    )
    .await;
    assert_eq!(run.code, 1, "{run:?}");
    for shown in ["MERGED", "fedcba9", "NOT_MERGEABLE", "It cannot merge now."] {
        assert!(run.stdout.contains(shown), "{shown}: {run:?}");
    }
    assert!(
        run.stderr.contains("1 of 2 pull requests were not merged"),
        "{run:?}"
    );
    assert!(
        run.stderr.contains("First") && run.stderr.contains("Second"),
        "{run:?}"
    );
    let targets = json!([
        { "repository": REPOSITORY, "number": 7, "expectedHeadSha": SHA },
        { "repository": REPOSITORY, "number": 8, "expectedHeadSha": OTHER_SHA },
    ]);
    let expected = json!({ "pullRequests": targets, "method": "REBASE" });
    assert_eq!(variables(&hub, "PullRequestMergeMany")[0], expected);
    assert!(
        variables(&hub, "PullRequestGet")
            .iter()
            .all(|sent| sent["fresh"] == true)
    );
}

#[tokio::test]
async fn pro_013_merging_several_never_takes_a_fork_or_the_flags_of_one() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "First", None));
    read_as(&hub, &fork_pull_request(9));
    let fork = ask(
        &hub,
        &[
            "pr", "merge", REPOSITORY, "7", "9", "--method", "merge", "--yes",
        ],
    )
    .await;
    assert_eq!(fork.code, 2, "{fork:?}");
    assert!(fork.stderr.contains("from a fork"), "{fork:?}");
    let many: Vec<String> = (1..=21).map(|n| n.to_string()).collect();
    let mut too_many = vec!["pr", "merge", REPOSITORY, "--method", "merge", "--yes"];
    too_many.extend(many.iter().map(String::as_str));
    let refused = [
        vec![
            "pr", "merge", REPOSITORY, "7", "7", "--method", "merge", "--yes",
        ],
        vec![
            "pr", "merge", REPOSITORY, "7", "8", "--method", "merge", "--sha", SHA, "--yes",
        ],
        vec![
            "pr", "merge", REPOSITORY, "7", "8", "--method", "merge", "--fork", "a/b", "--yes",
        ],
        vec![
            "pr",
            "merge",
            REPOSITORY,
            "7",
            "8",
            "--method",
            "merge",
            "--allow-failing-checks",
        ],
        vec!["pr", "merge", REPOSITORY, "7", "8", "--method", "merge"],
        too_many,
    ];
    for args in refused {
        assert_eq!(ask(&hub, &args).await.code, 2, "{args:?}");
    }
    assert_eq!(merges(&hub), 0);
}

#[tokio::test]
async fn cmd_021_each_hub_refusal_of_a_merge_has_its_exit_code() {
    let one = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", SHA, "--yes",
    ];
    let many = [
        "pr", "merge", REPOSITORY, "7", "8", "--method", "squash", "--yes",
    ];
    for (code, exit) in HUB_CODES {
        for (operation, args) in [
            ("PullRequestMergeOne", &one[..]),
            ("PullRequestMergeMany", &many[..]),
        ] {
            let hub = FakeHub::start().await;
            read_as(&hub, &pull_request(7, "Fix the thing", None));
            hub.answer(operation, refusal(code, "Refused by the test."));
            let run = ask(&hub, args).await;
            assert_eq!(run.code, exit, "{code} {operation}: {run:?}");
        }
    }
}
