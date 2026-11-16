//! `anima issue create`, `comment`, `close` and `reopen` against the scripted hub (PRJ-028,
//! OD-E14, CMD-021, CMD-025). The text of a body is sent, never printed back (LOG-007).

use serde_json::{Value, json};

use super::{ask, signed_in, variables};
use crate::kit::Outcome;
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::forge::{FULL_NAME, HUB_CODES, REPOSITORY, comment, issue, project};

const MARK: &str = "BODY-MARK-4711";

fn body() -> String {
    format!("Steps to reproduce:\n\n1. {MARK}\n")
}

/// Runs `anima <args> --hub <origin>` with `input` on standard input.
async fn with_input(hub: &FakeHub, args: &[&str], input: &str) -> Outcome {
    let mut all = args.to_vec();
    all.extend(["--hub", &hub.origin]);
    signed_in().run_with_input(&all, input).await
}

fn closed(reason: &str) -> Value {
    let mut closed = issue(12, "It breaks", Some(&body()));
    closed["open"] = json!(false);
    closed["closeReason"] = json!(reason);
    closed
}

#[tokio::test]
async fn prj_028_issue_create_sends_title_body_and_labels_and_never_prints_the_body() {
    let hub = FakeHub::start().await;
    project(&hub);
    let made = issue(12, "It breaks", Some(&body()));
    hub.answer("IssueCreate", data(json!({ "createIssue": made })));
    let folder = tempfile::tempdir().expect("a folder");
    let path = folder.path().join("body.md");
    std::fs::write(&path, body()).expect("a body file");
    let path = path.display().to_string();
    let args = [
        "issue",
        "create",
        FULL_NAME,
        "--title",
        "It breaks",
        "--body-file",
        &path,
        "--label",
        "bug",
        "--label",
        "p1",
    ];
    let run = ask(&hub, &args).await;
    assert_eq!(run.code, 0, "{run:?}");
    for shown in ["12", "It breaks", "help wanted", "issues/12"] {
        assert!(run.stdout.contains(shown), "{shown}: {run:?}");
    }
    run.assert_never_shows(MARK);
    let input = json!({ "repository": REPOSITORY, "title": "It breaks", "body": body(),
                        "labels": ["bug", "p1"] });
    assert_eq!(variables(&hub, "IssueCreate")[0], json!({ "input": input }));

    let args = [
        "issue",
        "create",
        REPOSITORY,
        "--title",
        "T",
        "--body-file",
        "-",
        "--json",
    ];
    let json = with_input(&hub, &args, &body()).await;
    assert_eq!(json.code, 0, "{json:?}");
    assert_eq!(json.json()["number"], 12);
    json.assert_never_shows(MARK);
    let sent = &variables(&hub, "IssueCreate")[1]["input"];
    assert_eq!(
        (&sent["body"], &sent["labels"]),
        (&json!(body()), &Value::Null)
    );

    let bare = ask(&hub, &["issue", "create", REPOSITORY, "--title", "T"]).await;
    assert_eq!(bare.code, 0, "{bare:?}");
    assert_eq!(
        variables(&hub, "IssueCreate")[2]["input"]["body"],
        Value::Null
    );
}

#[tokio::test]
async fn prj_028_issue_comment_sends_the_body_from_standard_input_and_prints_only_its_link() {
    let hub = FakeHub::start().await;
    let mut added = comment("901", &body());
    added["url"] = json!("https://github.com/acme/anima/issues/12#issuecomment-901");
    hub.answer("IssueCommentAdd", data(json!({ "addIssueComment": added })));
    let args = ["issue", "comment", REPOSITORY, "12", "--body-file", "-"];
    let run = with_input(&hub, &args, &body()).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("issuecomment-901"), "{run:?}");
    run.assert_never_shows(MARK);
    assert_eq!(
        variables(&hub, "IssueCommentAdd")[0],
        json!({ "repository": REPOSITORY, "number": 12, "body": body() })
    );

    let blank = with_input(&hub, &args, " \n").await;
    assert_eq!(blank.code, 2, "{blank:?}");
    let missing = [
        "issue",
        "comment",
        REPOSITORY,
        "12",
        "--body-file",
        "/nonexistent/x.md",
    ];
    let missing = ask(&hub, &missing).await;
    assert_eq!(missing.code, 1, "{missing:?}");
    let without = ask(&hub, &["issue", "comment", REPOSITORY, "12"]).await;
    assert_eq!(without.code, 2, "--body-file is required: {without:?}");
    assert_eq!(variables(&hub, "IssueCommentAdd").len(), 1);
}

#[tokio::test]
async fn prj_028_issue_close_and_reopen_send_open_and_the_reason() {
    let hub = FakeHub::start().await;
    hub.answer(
        "IssueSetOpen",
        data(json!({ "setIssueOpen": closed("NOT_PLANNED") })),
    );
    let run = ask(
        &hub,
        &[
            "issue",
            "close",
            REPOSITORY,
            "12",
            "--reason",
            "not-planned",
        ],
    )
    .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("closed (NOT_PLANNED)"), "{run:?}");
    run.assert_never_shows(MARK);
    let plain = ask(&hub, &["issue", "close", REPOSITORY, "12"]).await;
    assert_eq!(plain.code, 0, "{plain:?}");
    let reopen = ask(&hub, &["issue", "reopen", REPOSITORY, "12", "--json"]).await;
    assert_eq!(reopen.code, 0, "{reopen:?}");
    let sent = variables(&hub, "IssueSetOpen");
    let expected = [
        json!({ "repository": REPOSITORY, "number": 12, "open": false, "reason": "NOT_PLANNED" }),
        json!({ "repository": REPOSITORY, "number": 12, "open": false }),
        json!({ "repository": REPOSITORY, "number": 12, "open": true }),
    ];
    assert_eq!(sent, expected);
    let wrong = ask(
        &hub,
        &["issue", "close", REPOSITORY, "12", "--reason", "duplicate"],
    )
    .await;
    assert_eq!(wrong.code, 2, "duplicate is read, never written: {wrong:?}");
}

#[tokio::test]
async fn cmd_021_each_hub_refusal_of_an_issue_change_has_its_exit_code() {
    let commands: [(&str, &[&str]); 4] = [
        (
            "IssueCreate",
            &[
                "issue",
                "create",
                REPOSITORY,
                "--title",
                "T",
                "--body-file",
                "-",
            ],
        ),
        (
            "IssueCommentAdd",
            &["issue", "comment", REPOSITORY, "12", "--body-file", "-"],
        ),
        ("IssueSetOpen", &["issue", "close", REPOSITORY, "12"]),
        ("IssueSetOpen", &["issue", "reopen", REPOSITORY, "12"]),
    ];
    for (code, exit) in HUB_CODES {
        for (operation, args) in commands {
            let hub = FakeHub::start().await;
            hub.answer(operation, refusal(code, "Refused by the test."));
            let run = with_input(&hub, args, &body()).await;
            assert_eq!(run.code, exit, "{code} {args:?}: {run:?}");
            run.assert_never_shows(MARK);
        }
    }
}
