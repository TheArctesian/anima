//! `anima source connect github` and `anima source remove`: the private key stays out of
//! arguments and output, and a removal needs a stated choice and a confirmation (PRJ-018,
//! ITG-004, CMD-024, CMD-025, CMD-014).

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use serde_json::json;

use super::sources::{ID, account, listed, signed_in};
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::terminal::Terminal;

const KEY: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEAsecretsecretsecret\n-----END RSA PRIVATE KEY-----\n";

fn key_file(directory: &Path, mode: u32) -> String {
    let path = directory.join("app.pem");
    std::fs::write(&path, KEY).expect("a key file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("a mode");
    path.display().to_string()
}

fn connect<'a>(hub: &'a str, from: &'a str) -> Vec<&'a str> {
    vec![
        "source",
        "connect",
        "github",
        "--label",
        "acme",
        "--app-id",
        "123",
        "--installation-id",
        "456",
        "--from-file",
        from,
        "--hub",
        hub,
    ]
}

#[tokio::test]
async fn prj_018_connect_reads_the_key_from_a_0600_file_and_never_prints_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceConnect",
        data(json!({ "connectGithubApp": account("STARTING") })),
    );
    let directory = tempfile::tempdir().expect("a folder");
    let file = key_file(directory.path(), 0o600);
    let mut args = connect(&hub.origin, &file);
    args.push("--json");
    let run = signed_in().run(&args).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), account("STARTING"));
    run.assert_never_shows("MIIEowIBAAKCAQEAsecretsecretsecret");
    let input = &hub.seen()[0].variables["input"];
    assert_eq!(input["privateKey"], KEY);
    assert_eq!(
        (
            input["label"].clone(),
            input["appId"].clone(),
            input["installationId"].clone()
        ),
        (json!("acme"), json!("123"), json!("456"))
    );
}

#[tokio::test]
async fn cmd_025_connect_reads_the_key_from_standard_input_for_a_dash() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceConnect",
        data(json!({ "connectGithubApp": account("STARTING") })),
    );
    let run = signed_in()
        .run_with_input(&connect(&hub.origin, "-"), KEY)
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(hub.seen()[0].variables["input"]["privateKey"], KEY);
    run.assert_never_shows("MIIEowIBAAKCAQEAsecretsecretsecret");
}

#[tokio::test]
async fn cmd_025_connect_refuses_a_key_file_looser_than_0600_and_sends_nothing() {
    let hub = FakeHub::start().await;
    let directory = tempfile::tempdir().expect("a folder");
    let file = key_file(directory.path(), 0o644);
    let run = signed_in().run(&connect(&hub.origin, &file)).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("0600"), "{}", run.stderr);
    run.assert_never_shows("MIIEowIBAAKCAQEAsecretsecretsecret");
    assert!(hub.seen().is_empty(), "the hub was not asked");
}

#[tokio::test]
async fn cmd_025_connect_takes_no_flag_that_carries_the_key() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "source",
            "connect",
            "github",
            "--label",
            "acme",
            "--app-id",
            "123",
            "--installation-id",
            "456",
            "--private-key",
            "SECRETVALUE",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty());

    let run = signed_in()
        .run(&[
            "source",
            "connect",
            "github",
            "--label",
            "acme",
            "--app-id",
            "12x",
            "--installation-id",
            "456",
            "--from-file",
            "-",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "an App ID is digits only: {run:?}");
}

#[tokio::test]
async fn cmd_014_a_refusal_from_the_hub_does_not_repeat_the_key() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceConnect",
        refusal("VALIDATION", "GitHub refuses the key"),
    );
    let directory = tempfile::tempdir().expect("a folder");
    let file = key_file(directory.path(), 0o600);
    let run = signed_in().run(&connect(&hub.origin, &file)).await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.stderr.contains("GitHub refuses the key"),
        "{}",
        run.stderr
    );
    run.assert_never_shows("MIIEowIBAAKCAQEAsecretsecretsecret");
}

#[tokio::test]
async fn itg_004_remove_needs_keep_records_or_delete_records_and_exits_2_without() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&["source", "remove", "acme", "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    let run = signed_in()
        .run(&[
            "source",
            "remove",
            "acme",
            "--keep-records",
            "--delete-records",
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "the two exclude each other: {run:?}");
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn itg_004_remove_sends_the_chosen_records_flag_with_yes() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [listed("OK")] })),
    );
    hub.answer("SourceRemove", data(json!({ "removeSourceAccount": ID })));
    for (flag, delete) in [("--keep-records", false), ("--delete-records", true)] {
        let run = signed_in()
            .run(&[
                "source",
                "remove",
                "acme",
                flag,
                "--yes",
                "--json",
                "--hub",
                &hub.origin,
            ])
            .await;
        assert_eq!(run.code, 0, "{run:?}");
        assert_eq!(run.json(), json!(ID));
        let sent = hub.seen().pop().expect("sent");
        assert_eq!(sent.variables, json!({ "id": ID, "deleteRecords": delete }));
    }
}

#[tokio::test]
async fn cmd_024_remove_without_yes_or_a_terminal_sends_nothing() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [listed("OK")] })),
    );
    let run = signed_in()
        .run(&[
            "source",
            "remove",
            "acme",
            "--keep-records",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "SourceRemove")
    );
}

#[tokio::test]
async fn cmd_024_remove_names_the_account_and_a_no_sends_nothing() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SourceAccounts",
        data(json!({ "sourceAccounts": [listed("OK")] })),
    );
    let mut terminal = Terminal::start(
        &signed_in(),
        &[
            "source",
            "remove",
            "acme",
            "--delete-records",
            "--hub",
            &hub.origin,
        ],
    );
    terminal.expect("[y/N]").await;
    assert!(terminal.screen().contains("acme"), "{}", terminal.screen());
    terminal.type_line("n").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "SourceRemove")
    );
}
