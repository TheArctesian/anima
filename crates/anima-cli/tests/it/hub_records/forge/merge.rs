//! `anima pr merge` against the scripted hub (PRJ-029, PRJ-033, PRO-013, CMD-021, CMD-024): it
//! reads first, sends the head it showed, asks once, and a fork needs its name typed; `--yes`
//! never stands for that name.

use serde_json::{Value, json};

use super::{ask, signed_in, variables};
use crate::kit::fake_hub::{FakeHub, data};
use crate::kit::forge::{REPOSITORY, SHA, fork_pull_request, pull_request};
use crate::kit::terminal::Terminal;

pub const OTHER_SHA: &str = "fedcba9876543210fedcba9876543210fedcba98";

fn merged(number: i64) -> Value {
    let mut pull = pull_request(number, "Fix the thing", None);
    pull["state"] = json!("MERGED");
    pull
}

pub fn read_as(hub: &FakeHub, pull: &Value) {
    hub.answer("PullRequestGet", data(json!({ "pullRequest": pull })));
}

pub fn merges(hub: &FakeHub) -> usize {
    variables(hub, "PullRequestMergeOne").len() + variables(hub, "PullRequestMergeMany").len()
}

#[tokio::test]
async fn prj_033_merge_reads_fresh_sends_the_head_it_read_and_prints_one_line() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "Fix the thing", None));
    hub.answer(
        "PullRequestMergeOne",
        data(json!({ "mergePullRequest": merged(7) })),
    );
    let args = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", SHA, "--yes",
    ];
    let run = ask(&hub, &args).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("MERGED"), "{run:?}");
    for shown in [
        "Fix the thing",
        SHA,
        "CLEAN (can merge)",
        "PASSED: 3 passed",
        "squash",
    ] {
        assert!(run.stderr.contains(shown), "{shown}: {run:?}");
    }
    assert_eq!(variables(&hub, "PullRequestGet")[0]["fresh"], true);
    let input = json!({ "repository": REPOSITORY, "number": 7, "expectedHeadSha": SHA,
                        "method": "SQUASH", "allowFailingChecks": false });
    assert_eq!(
        variables(&hub, "PullRequestMergeOne")[0],
        json!({ "input": input })
    );

    let args = [
        "pr",
        "merge",
        REPOSITORY,
        "7",
        "--method",
        "merge",
        "--sha",
        SHA,
        "--yes",
        "--allow-failing-checks",
        "--json",
    ];
    let json = ask(&hub, &args).await;
    assert_eq!(json.json()["state"], "MERGED");
    let sent = &variables(&hub, "PullRequestMergeOne")[1]["input"];
    assert_eq!(
        (&sent["method"], &sent["allowFailingChecks"]),
        (&json!("MERGE"), &json!(true))
    );
}

#[tokio::test]
async fn prj_029_off_a_terminal_a_merge_needs_the_sha_it_saw_and_yes_or_sends_nothing() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "Fix the thing", None));
    let no_sha = ask(
        &hub,
        &[
            "pr", "merge", REPOSITORY, "7", "--method", "squash", "--yes",
        ],
    )
    .await;
    assert_eq!(no_sha.code, 2, "{no_sha:?}");
    assert!(no_sha.stderr.contains("--sha"), "{no_sha:?}");
    let no_yes = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", SHA,
    ];
    let no_yes = ask(&hub, &no_yes).await;
    assert_eq!(no_yes.code, 2, "{no_yes:?}");
    assert!(no_yes.stderr.contains("--yes"), "{no_yes:?}");
    let moved = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", OTHER_SHA, "--yes",
    ];
    let moved = ask(&hub, &moved).await;
    assert_eq!(moved.code, 6, "the head moved since: {moved:?}");
    let short = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", "0123456", "--yes",
    ];
    assert_eq!(ask(&hub, &short).await.code, 2);
    assert_eq!(merges(&hub), 0);
}

#[tokio::test]
async fn pro_013_yes_never_confirms_a_fork_and_only_its_exact_name_does() {
    let hub = FakeHub::start().await;
    read_as(&hub, &fork_pull_request(9));
    hub.answer(
        "PullRequestMergeOne",
        data(json!({ "mergePullRequest": merged(9) })),
    );
    let base = [
        "pr", "merge", REPOSITORY, "9", "--method", "merge", "--sha", SHA, "--yes",
    ];
    let yes_only = ask(&hub, &base).await;
    assert_eq!(yes_only.code, 2, "{yes_only:?}");
    assert!(yes_only.stderr.contains("--fork"), "{yes_only:?}");
    for wrong in ["acme/anima", "Mallory/anima", "mallory/anima2"] {
        let run = ask(&hub, &[&base[..], &["--fork", wrong]].concat()).await;
        assert_eq!(run.code, 5, "{wrong}: {run:?}");
    }
    assert_eq!(merges(&hub), 0);
    let right = ask(&hub, &[&base[..], &["--fork", "mallory/anima"]].concat()).await;
    assert_eq!(right.code, 0, "{right:?}");
    assert!(
        right.stderr.contains("From a fork: mallory/anima"),
        "{right:?}"
    );
    let sent = &variables(&hub, "PullRequestMergeOne")[0]["input"];
    assert_eq!(sent["forkConfirmed"], "mallory/anima");
}

#[tokio::test]
async fn pro_013_at_a_terminal_a_fork_name_typed_wrong_merges_nothing_even_with_yes() {
    let hub = FakeHub::start().await;
    read_as(&hub, &fork_pull_request(9));
    let args = [
        "pr",
        "merge",
        REPOSITORY,
        "9",
        "--method",
        "merge",
        "--sha",
        SHA,
        "--yes",
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("Type that name").await;
    terminal.type_line("mallory/anim").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 5, "{screen}");
    assert_eq!(merges(&hub), 0);
}

#[tokio::test]
async fn prj_029_at_a_terminal_no_sends_nothing_and_yes_sends_the_head_shown() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "Fix the thing", None));
    hub.answer(
        "PullRequestMergeOne",
        data(json!({ "mergePullRequest": merged(7) })),
    );
    let args = [
        "pr",
        "merge",
        REPOSITORY,
        "7",
        "--method",
        "rebase",
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("[y/N]").await;
    assert!(
        terminal.screen().contains("at 0123456 by rebase"),
        "{}",
        terminal.screen()
    );
    terminal.type_line("n").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert_eq!(merges(&hub), 0);

    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("[y/N]").await;
    terminal.type_line("y").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 0, "{screen}");
    assert_eq!(
        variables(&hub, "PullRequestMergeOne")[0]["input"]["expectedHeadSha"],
        SHA
    );
}

#[tokio::test]
async fn prj_033_a_pull_request_merged_before_or_closed_sends_nothing() {
    let hub = FakeHub::start().await;
    read_as(&hub, &merged(7));
    let args = [
        "pr", "merge", REPOSITORY, "7", "--method", "squash", "--sha", SHA, "--yes",
    ];
    let run = ask(&hub, &args).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("ALREADY_MERGED"), "{run:?}");
    let hub = FakeHub::start().await;
    let mut closed = pull_request(7, "Fix the thing", None);
    closed["state"] = json!("CLOSED");
    read_as(&hub, &closed);
    assert_eq!(ask(&hub, &args).await.code, 6);
    assert_eq!(merges(&hub), 0);
}

#[tokio::test]
async fn prj_033_with_json_at_a_terminal_the_summary_is_still_shown_before_the_question() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "Fix the thing", None));
    let args = [
        "pr",
        "merge",
        REPOSITORY,
        "7",
        "--method",
        "squash",
        "--json",
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("[y/N]").await;
    let screen = terminal.screen();
    terminal.type_line("n").await;
    let _ = terminal.finish().await;
    assert!(screen.contains("Fix the thing"), "{screen}");
    assert!(screen.contains("PASSED: 3 passed"), "{screen}");
    assert_eq!(merges(&hub), 0);
}

#[tokio::test]
async fn prj_033_yes_at_a_terminal_without_sha_sends_no_merge() {
    let hub = FakeHub::start().await;
    read_as(&hub, &pull_request(7, "Fix the thing", None));
    hub.answer(
        "PullRequestMergeOne",
        data(json!({ "mergePullRequest": merged(7) })),
    );
    let args = [
        "pr",
        "merge",
        REPOSITORY,
        "7",
        "--method",
        "squash",
        "--yes",
        "--hub",
        &hub.origin,
    ];
    let terminal = Terminal::start(&signed_in(), &args);
    let (code, screen) = terminal.finish().await;
    assert_eq!(
        merges(&hub),
        0,
        "merged a head no one confirmed (exit {code}): {screen}"
    );
}
