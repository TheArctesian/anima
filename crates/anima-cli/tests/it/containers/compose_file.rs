//! `anima containers check-file` and `from-run`, and the MCP tools `anima_compose_check` and
//! `anima_compose_from_run`, against the scripted hub (HOST-022, CMD-030, MCP-004).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};
use crate::mcp::session::{API_KEY, Desk, assert_protocol_only, hub};

const TOKEN: &str = "login-token-for-compose-tests";
const FILE: &str = "version: '3'\nservices:\n  web:\n    ports: 80\n";

/// The K1 `composeFileCheck` answer for [`FILE`].
fn found() -> Value {
    json!({ "composeFileCheck": { "syntaxValid": false, "cut": false, "problems": [
        { "code": "VERSION_OBSOLETE", "severity": "WARNING", "line": 1, "column": 1,
          "service": null, "key": null },
        { "code": "NO_IMAGE_OR_BUILD", "severity": "ERROR", "line": 3, "column": 3,
          "service": "web", "key": null },
        { "code": "WRONG_TYPE", "severity": "ERROR", "line": 4, "column": 5,
          "service": "web", "key": "ports" }
    ] } })
}

fn clean() -> Value {
    json!({ "composeFileCheck": { "syntaxValid": true, "cut": false, "problems": [] } })
}

const COMPOSE: &str = "services:\n  web:\n    image: \"nginx\"\n";

fn converted() -> Value {
    json!({ "composeFromDockerRun": { "composeFile": COMPOSE, "service": "web",
        "refusal": null, "refusedAt": null, "notes": ["DETACH_NOT_NEEDED"],
        "notConverted": ["--cidfile"] } })
}

fn refused() -> Value {
    json!({ "composeFromDockerRun": { "composeFile": null, "service": null,
        "refusal": "PIPE", "refusedAt": 24, "notes": [], "notConverted": [] } })
}

async fn anima(hub: &FakeHub, words: &[&str], input: &str) -> crate::kit::Outcome {
    // `--hub` goes before the words: each word after `--` is a word of the command.
    let mut line = vec!["--hub", hub.origin.as_str()];
    line.extend(words);
    Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run_with_input(&line, input)
        .await
}

/// The variables of the last request of `operation`.
fn sent(hub: &FakeHub, operation: &str) -> Value {
    let seen = hub.seen();
    let last = seen.iter().rev().find(|seen| seen.operation == operation);
    last.expect("a request").variables.clone()
}

#[tokio::test]
async fn host_022_check_file_shows_each_problem_and_ends_with_1_on_an_error() {
    let hub = FakeHub::start().await;
    hub.answer("ComposeFileCheckRun", data(found()))
        .answer("ComposeFileCheckRun", data(clean()))
        .answer("ComposeFileCheckRun", data(found()));
    let run = anima(&hub, &["containers", "check-file", "-"], FILE).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert_eq!(sent(&hub, "ComposeFileCheckRun"), json!({ "file": FILE }));
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert!(lines[0].starts_with("LINE"), "{}", run.stdout);
    assert!(lines[1].contains("version obsolete"), "{}", run.stdout);
    assert!(
        lines[3].contains("wrong type") && lines[3].ends_with("ports"),
        "{}",
        run.stdout
    );
    assert!(run.stderr.contains("syntax only"), "{}", run.stderr);
    assert!(run.stderr.contains("errors of syntax: 2"), "{}", run.stderr);

    let run = anima(&hub, &["containers", "check-file", "-"], COMPOSE).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, "No problem found.\n");

    let run = anima(&hub, &["containers", "check-file", "-", "--json"], FILE).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert_eq!(run.json(), found()["composeFileCheck"]);
}

#[tokio::test]
async fn host_022_check_file_refuses_a_file_larger_than_the_hub_reads() {
    let hub = FakeHub::start().await;
    let large = "#".repeat(256 * 1024 + 1);
    let run = anima(&hub, &["containers", "check-file", "-"], &large).await;
    assert_ne!(run.code, 0, "{run:?}");
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn host_022_from_run_prints_the_file_and_the_notes_on_standard_error() {
    let hub = FakeHub::start().await;
    hub.answer("ComposeFromDockerRunRead", data(converted()));
    let words = [
        "containers",
        "from-run",
        "--",
        "docker",
        "run",
        "-e",
        "A=it's $5",
        "nginx",
    ];
    let run = anima(&hub, &words, "").await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, COMPOSE);
    // The shell already read each word: each goes to the hub in single quotes.
    let command = r"'docker' 'run' '-e' 'A=it'\''s $5' 'nginx'";
    assert_eq!(
        sent(&hub, "ComposeFromDockerRunRead"),
        json!({ "command": command })
    );
    assert!(run.stderr.contains("`-d` is left out"), "{}", run.stderr);
    assert!(run.stderr.contains("--cidfile"), "{}", run.stderr);

    let line = "docker run -d \\\n  nginx\n";
    let run = anima(&hub, &["containers", "from-run"], line).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        sent(&hub, "ComposeFromDockerRunRead"),
        json!({ "command": line })
    );
}

#[tokio::test]
async fn host_022_from_run_says_why_a_command_was_not_converted() {
    let hub = FakeHub::start().await;
    hub.answer("ComposeFromDockerRunRead", data(refused()));
    let line = "docker run nginx | tee out";
    let run = anima(&hub, &["containers", "from-run"], line).await;
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.contains("The command has a pipe.") && run.stderr.contains("character 24"),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn host_022_the_compose_tools_give_the_problems_and_the_file() {
    let hub = hub().await;
    hub.answer("ComposeFileCheckRun", data(found()));
    hub.answer("ComposeFromDockerRunRead", data(converted()));
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &[]).await;
    let called = session
        .call("anima_compose_check", json!({ "file": FILE }))
        .await;
    assert_eq!(called["isError"], false, "{called}");
    let check = &called["structuredContent"];
    assert_eq!(check["syntaxValid"], false);
    assert_eq!(check["errors"], 2);
    assert_eq!(check["problems"][2]["key"], "ports");
    let called = session
        .call(
            "anima_compose_from_run",
            json!({ "command": "docker run -d nginx" }),
        )
        .await;
    assert_eq!(called["isError"], false, "{called}");
    let file = &called["structuredContent"];
    // The tool cuts the last line break, as it does for each text with lines.
    assert_eq!(file["composeFile"], COMPOSE.trim_end());
    assert_eq!(file["notes"][0]["code"], "DETACH_NOT_NEEDED");
    assert_eq!(file["notConverted"], json!(["--cidfile"]));
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, API_KEY);
}
