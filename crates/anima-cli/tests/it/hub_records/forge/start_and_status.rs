//! `anima workflow start` and `anima repo status set` against the scripted hub (PRJ-035,
//! PRO-010, CMD-021, CMD-024). An input's value is sent and never shown (LOG-007).

use serde_json::{Value, json};

use super::{ask, signed_in, variables};
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::forge::{HUB_CODES, REPOSITORY, SHA, workflow};
use crate::kit::terminal::Terminal;

const VALUE: &str = "VALUE-MARK-9931";

fn accepted() -> Value {
    data(
        json!({ "dispatchWorkflow": { "repository": REPOSITORY, "workflowId": "42",
                 "branch": "main", "acceptedAt": "2026-11-14T10:00:00Z" } }),
    )
}

/// A hub that lists the workflow `ci.yml` and answers `WorkflowStart` with `start`.
async fn scripted(start: Value) -> FakeHub {
    let hub = FakeHub::start().await;
    hub.answer("WorkflowList", data(json!({ "workflows": [workflow()] })));
    hub.answer("WorkflowStart", start);
    hub
}

fn input(value: &str) -> String {
    format!("token={value}")
}

#[tokio::test]
async fn prj_035_workflow_start_finds_the_workflow_by_path_or_id_and_never_shows_a_value() {
    let hub = scripted(accepted()).await;
    let token = input(VALUE);
    let args = [
        "workflow",
        "start",
        REPOSITORY,
        ".github/workflows/ci.yml",
        "--branch",
        "main",
        "--input",
        "tag=v1.2",
        "--input",
        &token,
        "--yes",
    ];
    let run = ask(&hub, &args).await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(VALUE);
    assert!(
        run.stdout.contains("CI (.github/workflows/ci.yml)"),
        "{run:?}"
    );
    assert!(run.stderr.contains("anima workflow runs"), "{run:?}");
    let inputs = json!([{ "name": "tag", "value": "v1.2" }, { "name": "token", "value": VALUE }]);
    let input = json!({ "repository": REPOSITORY, "workflowId": "42", "branch": "main",
                        "inputs": inputs });
    assert_eq!(
        variables(&hub, "WorkflowStart")[0],
        json!({ "input": input })
    );
    assert_eq!(variables(&hub, "WorkflowList")[0]["fresh"], true);

    let by_id = [
        "workflow", "start", REPOSITORY, "42", "--branch", "main", "--yes", "--json",
    ];
    let json = ask(&hub, &by_id).await;
    assert_eq!(json.json()["acceptedAt"], "2026-11-14T10:00:00Z");
    assert_eq!(
        variables(&hub, "WorkflowStart")[1]["input"]["inputs"],
        json!([])
    );
}

#[tokio::test]
async fn prj_035_a_bad_input_an_unknown_or_disabled_workflow_or_no_yes_sends_nothing() {
    let hub = scripted(accepted()).await;
    let start = |workflow: &'static str, extra: &[&'static str]| {
        let mut args = vec![
            "workflow", "start", REPOSITORY, workflow, "--branch", "main",
        ];
        args.extend(extra);
        args
    };
    let bare = format!("{VALUE}-without-a-name");
    let bad = ask(&hub, &start("42", &["--input", "x", "--yes"])).await;
    assert_eq!(bad.code, 2, "{bad:?}");
    let mut args = start("42", &["--yes", "--input"]);
    args.push(&bare);
    let hidden = ask(&hub, &args).await;
    assert_eq!(hidden.code, 2, "{hidden:?}");
    hidden.assert_never_shows(VALUE);
    assert_eq!(
        variables(&hub, "WorkflowList").len(),
        0,
        "inputs are checked first"
    );
    assert_eq!(ask(&hub, &start("7", &["--yes"])).await.code, 3);
    assert_eq!(
        ask(&hub, &start("42", &[])).await.code,
        2,
        "no --yes off a terminal"
    );
    let disabled_hub = FakeHub::start().await;
    let mut disabled = workflow();
    disabled["active"] = json!(false);
    disabled_hub.answer("WorkflowList", data(json!({ "workflows": [disabled] })));
    assert_eq!(ask(&disabled_hub, &start("42", &["--yes"])).await.code, 6);
    assert_eq!(variables(&hub, "WorkflowStart").len(), 0);
    assert_eq!(variables(&disabled_hub, "WorkflowStart").len(), 0);
}

#[tokio::test]
async fn prj_035_the_question_names_the_inputs_but_not_their_values() {
    let hub = scripted(accepted()).await;
    let token = input(VALUE);
    let args = [
        "workflow",
        "start",
        REPOSITORY,
        "42",
        "--branch",
        "main",
        "--input",
        &token,
        "--hub",
        &hub.origin,
    ];
    let mut terminal = Terminal::start(&signed_in(), &args);
    terminal.expect("[y/N]").await;
    let screen = terminal.screen();
    assert!(screen.contains("with the inputs token"), "{screen}");
    terminal.type_line("n").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert!(!screen.contains(VALUE), "{screen}");
    assert_eq!(variables(&hub, "WorkflowStart").len(), 0);
}

#[tokio::test]
async fn pro_010_repo_status_set_sends_the_status_under_its_context() {
    let hub = FakeHub::start().await;
    let status = json!({ "repository": REPOSITORY, "sha": SHA, "context": "anima/build",
                         "state": "SUCCESS", "description": "Built", "targetUrl": null,
                         "createdAt": "2026-11-14T10:00:00Z" });
    hub.answer(
        "CommitStatusSet",
        data(json!({ "setCommitStatus": status })),
    );
    let args = [
        "repo",
        "status",
        "set",
        REPOSITORY,
        SHA,
        "--state",
        "success",
        "--context",
        "anima/build",
        "--description",
        "Built",
        "--url",
        "https://hub.example/b/1",
    ];
    let run = ask(&hub, &args).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("anima/build"), "{run:?}");
    let input = json!({ "repository": REPOSITORY, "sha": SHA, "state": "SUCCESS",
                        "context": "anima/build", "description": "Built",
                        "targetUrl": "https://hub.example/b/1" });
    assert_eq!(
        variables(&hub, "CommitStatusSet")[0],
        json!({ "input": input })
    );
    let bare = [
        "repo",
        "status",
        "set",
        REPOSITORY,
        SHA,
        "--state",
        "pending",
        "--context",
        "anima/build",
        "--json",
    ];
    let json = ask(&hub, &bare).await;
    assert_eq!(json.json()["context"], "anima/build");
    let sent = &variables(&hub, "CommitStatusSet")[1]["input"];
    assert_eq!(
        (&sent["description"], &sent["targetUrl"]),
        (&Value::Null, &Value::Null)
    );
}

#[tokio::test]
async fn cmd_021_each_hub_refusal_of_a_start_or_a_status_has_its_exit_code() {
    let token = input(VALUE);
    let start = [
        "workflow", "start", REPOSITORY, "42", "--branch", "main", "--input", &token, "--yes",
    ];
    let status = [
        "repo",
        "status",
        "set",
        REPOSITORY,
        SHA,
        "--state",
        "error",
        "--context",
        "anima/x",
    ];
    for (code, exit) in HUB_CODES {
        for (operation, args) in [
            ("WorkflowStart", &start[..]),
            ("CommitStatusSet", &status[..]),
        ] {
            let refused = refusal(code, "Refused by the test.");
            let hub = scripted(refused.clone()).await;
            hub.answer("CommitStatusSet", refused);
            let run = ask(&hub, args).await;
            assert_eq!(run.code, exit, "{code} {operation}: {run:?}");
            run.assert_never_shows(VALUE);
        }
    }
}
