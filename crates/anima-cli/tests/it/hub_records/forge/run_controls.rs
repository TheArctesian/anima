//! `anima workflow rerun` and `anima workflow cancel` against the scripted hub (PRJ-029,
//! PRJ-034, CMD-021, CMD-024): each reads the run fresh, names its commit in the question and
//! sends that commit.

use serde_json::{Value, json};

use super::{ask, signed_in, variables};
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::forge::{HUB_CODES, REPOSITORY, SHA, run};
use crate::kit::terminal::Terminal;

const CHANGES: [&str; 3] = [
    "WorkflowRunRerun",
    "WorkflowRunRerunFailed",
    "WorkflowRunCancel",
];

fn running() -> Value {
    let mut running = run("5001", "Break it");
    running["status"] = json!("IN_PROGRESS");
    running["conclusion"] = Value::Null;
    running["rerunnable"] = json!(false);
    running["failedJobsRerunnable"] = json!(false);
    running["cancellable"] = json!(true);
    running
}

fn read_as(hub: &FakeHub, found: &Value) {
    hub.answer("WorkflowRunGet", data(json!({ "workflowRun": found })));
}

/// Scripts each change to answer with `answer`.
fn changes_answer(hub: &FakeHub, answer: &Value) {
    let roots = [
        "rerunWorkflowRun",
        "rerunFailedWorkflowJobs",
        "cancelWorkflowRun",
    ];
    for (operation, root) in CHANGES.iter().zip(roots) {
        hub.answer(operation, data(json!({ root: answer })));
    }
}

fn sent_changes(hub: &FakeHub) -> usize {
    CHANGES
        .iter()
        .map(|operation| variables(hub, operation).len())
        .sum()
}

#[tokio::test]
async fn prj_034_rerun_rerun_failed_and_cancel_send_the_head_of_the_run_they_read() {
    let hub = FakeHub::start().await;
    read_as(&hub, &run("5001", "Break it"));
    changes_answer(&hub, &run("5001", "Break it"));
    let rerun = ask(&hub, &["workflow", "rerun", REPOSITORY, "5001", "--yes"]).await;
    assert_eq!(rerun.code, 0, "{rerun:?}");
    assert!(rerun.stdout.contains("5001"), "{rerun:?}");
    let failed = [
        "workflow", "rerun", REPOSITORY, "5001", "--failed", "--yes", "--json",
    ];
    let failed = ask(&hub, &failed).await;
    assert_eq!(failed.json()["headSha"], SHA);
    let expected = json!({ "repository": REPOSITORY, "runId": "5001", "expectedHeadSha": SHA });
    assert_eq!(
        variables(&hub, "WorkflowRunRerun"),
        std::slice::from_ref(&expected)
    );
    assert_eq!(
        variables(&hub, "WorkflowRunRerunFailed"),
        std::slice::from_ref(&expected)
    );

    let hub = FakeHub::start().await;
    read_as(&hub, &running());
    changes_answer(&hub, &running());
    let cancel = ask(&hub, &["workflow", "cancel", REPOSITORY, "5001", "--yes"]).await;
    assert_eq!(cancel.code, 0, "{cancel:?}");
    assert_eq!(variables(&hub, "WorkflowRunCancel"), [expected]);
    assert!(
        variables(&hub, "WorkflowRunGet")
            .iter()
            .all(|sent| sent["fresh"] == true)
    );
}

#[tokio::test]
async fn prj_034_a_run_that_cannot_change_so_is_refused_before_anything_is_sent() {
    let hub = FakeHub::start().await;
    read_as(&hub, &running());
    for args in [
        ["workflow", "rerun", REPOSITORY, "5001", "--yes"],
        ["workflow", "rerun", REPOSITORY, "5001", "--failed"],
    ] {
        assert_eq!(ask(&hub, &args).await.code, 6, "{args:?}");
    }
    let hub_finished = FakeHub::start().await;
    let mut forked = run("5001", "Break it");
    forked["fromFork"] = json!(true);
    forked["rerunnable"] = json!(false);
    read_as(&hub_finished, &forked);
    let cancel = ask(
        &hub_finished,
        &["workflow", "cancel", REPOSITORY, "5001", "--yes"],
    )
    .await;
    assert_eq!(
        cancel.code, 6,
        "a finished run is not cancellable: {cancel:?}"
    );
    let rerun = ask(
        &hub_finished,
        &["workflow", "rerun", REPOSITORY, "5001", "--yes"],
    )
    .await;
    assert_eq!(
        rerun.code, 6,
        "a fork's run is not run again (PRO-013): {rerun:?}"
    );
    let missing_hub = FakeHub::start().await;
    missing_hub.answer("WorkflowRunGet", data(json!({ "workflowRun": null })));
    let missing = ask(
        &missing_hub,
        &["workflow", "cancel", REPOSITORY, "1", "--yes"],
    )
    .await;
    assert_eq!(missing.code, 3, "{missing:?}");
    assert_eq!(
        sent_changes(&hub) + sent_changes(&hub_finished) + sent_changes(&missing_hub),
        0
    );
}

#[tokio::test]
async fn prj_029_without_yes_or_a_yes_at_the_terminal_nothing_is_sent() {
    let hub = FakeHub::start().await;
    read_as(&hub, &run("5001", "Break it"));
    let piped = ask(&hub, &["workflow", "rerun", REPOSITORY, "5001"]).await;
    assert_eq!(piped.code, 2, "{piped:?}");
    assert!(piped.stderr.contains("--yes"), "{piped:?}");
    let args = [
        "workflow",
        "rerun",
        REPOSITORY,
        "5001",
        "--failed",
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("[y/N]").await;
    let screen = terminal.screen();
    for shown in ["failed jobs", "CI #9", "0123456", "Break it"] {
        assert!(screen.contains(shown), "{shown}: {screen}");
    }
    terminal.type_line("no").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert_eq!(sent_changes(&hub), 0);
}

#[tokio::test]
async fn cmd_021_each_hub_refusal_of_a_run_control_has_its_exit_code() {
    let commands: [(&str, &[&str], Value); 3] = [
        (
            "WorkflowRunRerun",
            &["workflow", "rerun", REPOSITORY, "5001", "--yes"],
            run("5001", "x"),
        ),
        (
            "WorkflowRunRerunFailed",
            &["workflow", "rerun", REPOSITORY, "5001", "--failed", "--yes"],
            run("5001", "x"),
        ),
        (
            "WorkflowRunCancel",
            &["workflow", "cancel", REPOSITORY, "5001", "--yes"],
            running(),
        ),
    ];
    for (code, exit) in HUB_CODES {
        for (operation, args, found) in &commands {
            let hub = FakeHub::start().await;
            read_as(&hub, found);
            hub.answer(operation, refusal(code, "Refused by the test."));
            let outcome = ask(&hub, args).await;
            assert_eq!(outcome.code, exit, "{code} {operation}: {outcome:?}");
        }
    }
}
