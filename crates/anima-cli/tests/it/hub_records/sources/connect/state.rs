//! `--state`, `--cancel` and the flags that exclude each other (PRJ-041, CMD-021).

use serde_json::json;

use super::{ID, INSTALL, app_created, asked, attempt, connect_github, scripted};
use crate::kit::fake_hub::{FakeHub, data};
use crate::kit::subscription_hub::Script;

#[tokio::test]
async fn prj_041_state_prints_the_open_attempt_or_that_none_is() {
    let (hub, _) = scripted(Script::Next(vec![])).await;
    hub.answer(
        "GithubConnectState",
        data(json!({ "githubConnect": app_created() })),
    );
    let shown = connect_github(&hub, &["--state"]).await;
    assert_eq!(shown.code, 0, "{shown:?}");
    assert!(
        shown.stdout.starts_with("APP_CREATED: ") && shown.stdout.contains(INSTALL),
        "{shown:?}"
    );
    let json = connect_github(&hub, &["--state", "--json"]).await;
    assert_eq!(json.json()["phase"], "APP_CREATED");
    let (hub, _) = scripted(Script::Next(vec![])).await;
    hub.answer("GithubConnectState", data(json!({ "githubConnect": null })));
    let none = connect_github(&hub, &["--state"]).await;
    assert_eq!(none.stdout.trim(), "No GitHub connection is in progress.");
    assert_eq!(asked(&hub, "GithubConnectStart"), 0);
}

#[tokio::test]
async fn prj_041_cancel_cancels_the_open_attempt_and_says_when_none_is_open() {
    let (hub, _) = scripted(Script::Next(vec![])).await;
    hub.answer(
        "GithubConnectState",
        data(json!({ "githubConnect": attempt("AT_GITHUB") })),
    );
    let mut cancelled = attempt("CANCELLED");
    cancelled["message"] = json!("Cancelled.");
    hub.answer(
        "GithubConnectCancel",
        data(json!({ "cancelGithubConnect": cancelled })),
    );
    let run = connect_github(&hub, &["--cancel"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout.trim(), "Cancelled.");
    let sent = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "GithubConnectCancel");
    assert_eq!(sent.expect("sent").variables["id"], ID);

    let (hub, _) = scripted(Script::Next(vec![])).await;
    hub.answer("GithubConnectState", data(json!({ "githubConnect": null })));
    let none = connect_github(&hub, &["--cancel"]).await;
    assert_eq!(none.stdout.trim(), "No GitHub connection is in progress.");
    assert_eq!(asked(&hub, "GithubConnectCancel"), 0);
}

#[tokio::test]
async fn cmd_021_flags_that_exclude_each_other_are_usage_errors() {
    let hub = FakeHub::start().await;
    let key_file = ["--from-file", "-", "--label", "x"];
    for args in [
        vec!["--app-id", "1"],
        vec!["--installation-id", "2"],
        vec!["--state", "--cancel"],
        vec!["--from-file", "-"],
        [&key_file[..], &["--app-id", "1"]].concat(),
        vec!["--state", "--label", "x"],
        [
            &key_file[..],
            &["--app-id", "1", "--installation-id", "2", "--no-browser"],
        ]
        .concat(),
    ] {
        let run = connect_github(&hub, &args).await;
        assert_eq!(run.code, 2, "{args:?}: {run:?}");
    }
    assert!(hub.seen().is_empty());
}
