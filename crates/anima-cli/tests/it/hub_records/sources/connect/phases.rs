//! The phases the command prints and the exit code of each ending (PRJ-039, PRJ-041, CMD-021).

use std::process::Stdio;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncBufReadExt as _, BufReader};

use super::{
    DELETE, INSTALL, PAGE, TICKET, app_created, asked, attempt, changes, connect_github, connected,
    failed, made, scripted, scripted_start, signed_in,
};
use crate::kit::fake_hub::{data, refusal};
use crate::kit::subscription_hub::Script;

#[tokio::test]
async fn prj_039_off_a_terminal_the_command_prints_the_page_and_each_phase_and_never_asks_for_the_link()
 {
    let script = Script::Next(changes(&[attempt("AT_GITHUB"), app_created(), connected()]));
    let (hub, socket) = scripted(script).await;
    let run = connect_github(&hub, &["--label", "mine", "--organisation", "acme"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(
        lines,
        [
            "Open this page in a browser that is signed in to Anima and press Continue.",
            &format!("  {PAGE}"),
            "Waiting for you to press \"Create GitHub App\" on GitHub.",
            "GitHub App anima-1a2b is made for me. Waiting for you to press \"Install\".",
            &format!("  {INSTALL}"),
            "GitHub is connected as \"mine\". The first sync has started.",
        ]
    );
    assert!(
        run.stdout.contains("mine") && run.stdout.contains("github"),
        "{run:?}"
    );
    assert_eq!(
        asked(&hub, "GithubConnectOpen"),
        0,
        "no one-time link was asked for"
    );
    let start = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "GithubConnectStart");
    let input = &start.expect("started").variables["input"];
    assert_eq!(input, &json!({ "label": "mine", "organisation": "acme" }));
    assert!(socket.received().init["authorization"].is_string());
    assert!(!run.stdout.contains(TICKET) && !run.stderr.contains(TICKET));
}

#[tokio::test]
async fn prj_041_json_prints_the_final_attempt_and_no_link() {
    let (hub, _) = scripted(Script::Next(changes(&[attempt("AT_GITHUB"), connected()]))).await;
    let run = connect_github(&hub, &["--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), connected());
    assert!(
        !run.stdout.contains("ticket") && !run.stderr.contains("ticket"),
        "{run:?}"
    );
}

#[tokio::test]
async fn prj_041_each_failure_has_its_exit_code_its_words_and_the_delete_link() {
    let cases = [
        (
            failed(
                "EXPIRED",
                "The connection was not finished in time. Press Connect GitHub to start again.",
                false,
            ),
            1,
            "not finished in time",
            false,
        ),
        (
            failed(
                "KEY_REFUSED",
                "GitHub refused the new App's key. Delete the App anima-1a2b on GitHub, then start again.",
                true,
            ),
            1,
            "refused the new App's key",
            true,
        ),
        (
            failed(
                "GITHUB_UNREACHABLE",
                "The hub could not reach GitHub to finish making the App. Start again.",
                true,
            ),
            7,
            "could not reach GitHub",
            true,
        ),
    ];
    for (over, code, words, delete) in cases {
        let (hub, _) = scripted(Script::Next(changes(&[attempt("AT_GITHUB"), over]))).await;
        let run = connect_github(&hub, &[]).await;
        assert_eq!(run.code, code, "{run:?}");
        assert!(run.stderr.contains(words), "{run:?}");
        assert_eq!(run.stderr.contains(DELETE), delete, "{run:?}");
        assert_eq!(run.stdout, "");
    }
}

#[tokio::test]
async fn prj_039_a_connection_cancelled_elsewhere_ends_with_exit_1_and_its_words() {
    let mut cancelled = made(attempt("CANCELLED"));
    cancelled["message"] =
        json!("Cancelled. The App anima-1a2b is still on GitHub. Delete it there.");
    cancelled["appSettingsUrl"] = json!(DELETE);
    let (hub, _) = scripted(Script::Next(changes(&[cancelled]))).await;
    let run = connect_github(&hub, &[]).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("Cancelled. The App anima-1a2b") && run.stderr.contains(DELETE));
}

#[tokio::test]
async fn prj_041_a_hub_that_forgot_the_attempt_says_it_restarted() {
    let (hub, _) = scripted(Script::Next(vec![])).await;
    hub.answer("GithubConnectState", data(json!({ "githubConnect": null })));
    let run = connect_github(&hub, &[]).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(
        run.stderr
            .contains("The hub restarted and dropped the connection."),
        "{run:?}"
    );
}

#[tokio::test]
async fn cmd_021_a_conflict_at_start_is_exit_6_and_an_api_key_is_exit_5() {
    for (code, exit, words) in [
        (
            "CONFLICT",
            6,
            "A GitHub App is made and waits for Install. Finish it or cancel it first.",
        ),
        ("FORBIDDEN", 5, "Closed to API keys."),
    ] {
        let (hub, _) = scripted_start(Script::Next(vec![]), refusal(code, words)).await;
        let run = connect_github(&hub, &[]).await;
        assert_eq!(run.code, exit, "{run:?}");
        assert!(run.stderr.contains(words), "{run:?}");
        assert_eq!(asked(&hub, "GithubConnectOpen"), 0);
    }
}

#[tokio::test]
async fn prj_041_ctrl_c_leaves_the_attempt_open_and_exits_130_with_the_cancel_command() {
    let (hub, _) = scripted(Script::Hold(changes(&[attempt("AT_GITHUB")]))).await;
    let mut command = signed_in().command(env!("CARGO_BIN_EXE_anima"));
    let mut child = command
        .args(["source", "connect", "github", "--hub", &hub.origin])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("starts");
    let mut lines = BufReader::new(child.stderr.take().expect("stderr")).lines();
    let waiting = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(line) = lines.next_line().await.expect("a line") {
            if line.contains("Create GitHub App") {
                return;
            }
        }
        panic!("the phase line never came");
    })
    .await;
    waiting.expect("the first phase line");
    let pid = child.id().expect("running").to_string();
    let sent = tokio::process::Command::new("/bin/kill")
        .args(["-INT", &pid])
        .status()
        .await
        .expect("kill runs");
    assert!(sent.success());
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("it stops")
        .expect("a status");
    let mut rest = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        rest.push_str(&line);
    }
    assert_eq!(status.code(), Some(130), "{status:?} {rest}");
    assert!(
        rest.contains("still open on the hub") && rest.contains("--cancel"),
        "{rest}"
    );
}

#[tokio::test]
async fn prj_041_text_from_github_is_cleaned_and_foreign_links_are_not_printed() {
    let mut hostile = app_created();
    hostile["appSlug"] = json!("anima\u{1b}[2J-x");
    hostile["ownerLogin"] = json!("me\u{7}\u{202E}");
    hostile["installUrl"] = json!("https://evil.example/install");
    let (hub, _) = scripted(Script::Next(changes(&[hostile, connected()]))).await;
    let run = connect_github(&hub, &[]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        !run.stderr.contains(['\u{1b}', '\u{7}', '\u{202E}']),
        "{:?}",
        run.stderr
    );
    assert!(run.stderr.contains("anima [2J-x"), "{:?}", run.stderr);
    assert!(!run.stderr.contains("evil"), "{:?}", run.stderr);
}
