//! `anima builder` against the scripted hub (BLD-023, BLD-025, CMD-014, CMD-024).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-builder-tests";
const BUILDER_TOKEN: &str = "builder-token-6d2e8f0a1b3c4d5e";
const ID: &str = "0190a000-0000-7000-9001-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn builder() -> Value {
    json!({
        "id": ID, "name": "mac-mini", "platform": { "os": "MACOS", "arch": "AARCH64" },
        "clientAddr": "100.64.0.20", "enrolledAt": "2026-11-01T09:00:00+00:00",
        "lastSeenAt": null, "online": false,
    })
}

#[tokio::test]
async fn builder_list_prints_a_table_and_json() {
    let hub = FakeHub::start().await;
    hub.answer("BuilderList", data(json!({ "builders": [builder()] })));
    let run = signed_in()
        .run(&["builder", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run.stdout.lines().nth(1).expect("a row");
    assert!(
        row.starts_with("mac-mini") && row.contains("MACOS AARCH64"),
        "{row}"
    );
    let run = signed_in()
        .run(&["builder", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()[0]["clientAddr"], "100.64.0.20");
}

#[tokio::test]
async fn bld_023_enrol_prints_the_token_once_on_stdout_with_a_note_on_stderr() {
    let hub = FakeHub::start().await;
    let enrolment = json!({
        "builder": builder(), "token": BUILDER_TOKEN, "expiresAt": "2027-11-01T09:00:00+00:00",
    });
    hub.answer("BuilderEnrol", data(json!({ "enrolBuilder": enrolment })));
    let run = signed_in()
        .run(&[
            "builder",
            "enrol",
            "--name",
            "mac-mini",
            "--os",
            "macos",
            "--arch",
            "aarch64",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, format!("{BUILDER_TOKEN}\n"));
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(!run.stderr.contains(BUILDER_TOKEN), "{}", run.stderr);
    let seen = hub.seen();
    assert_eq!(
        seen[0].variables["platform"],
        json!({ "os": "MACOS", "arch": "AARCH64" })
    );
}

#[tokio::test]
async fn cmd_014_enrol_to_file_keeps_the_token_off_stdout_and_stderr() {
    let hub = FakeHub::start().await;
    let enrolment = json!({
        "builder": builder(), "token": BUILDER_TOKEN, "expiresAt": "2027-11-01T09:00:00+00:00",
    });
    hub.answer("BuilderEnrol", data(json!({ "enrolBuilder": enrolment })));
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("builder.token");
    let run = signed_in()
        .run(&[
            "builder",
            "enrol",
            "--name",
            "mac-mini",
            "--os",
            "macos",
            "--arch",
            "aarch64",
            "--to-file",
            path.to_str().expect("a path"),
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(BUILDER_TOKEN);
    assert_eq!(
        std::fs::read_to_string(&path).expect("a file"),
        BUILDER_TOKEN
    );
}

#[tokio::test]
async fn cmd_024_revoke_by_name_needs_yes_and_then_deletes_by_id() {
    let hub = FakeHub::start().await;
    hub.answer("BuilderList", data(json!({ "builders": [builder()] })))
        .answer("BuilderRevoke", data(json!({ "revokeBuilder": ID })));
    let refused = signed_in()
        .run(&["builder", "revoke", "mac-mini", "--hub", &hub.origin])
        .await;
    assert_eq!(refused.code, 2, "{refused:?}");
    assert!(refused.stderr.contains("--yes"), "{}", refused.stderr);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "BuilderRevoke")
    );

    let run = signed_in()
        .run(&[
            "builder",
            "revoke",
            "mac-mini",
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let seen = hub.seen();
    let revoke = seen
        .iter()
        .find(|s| s.operation == "BuilderRevoke")
        .expect("a call");
    assert_eq!(revoke.variables["id"], ID);
}

#[tokio::test]
async fn builder_revoke_of_an_unknown_builder_exits_3_and_a_forbidden_enrol_exits_5() {
    let hub = FakeHub::start().await;
    hub.answer("BuilderList", data(json!({ "builders": [] })));
    let run = signed_in()
        .run(&["builder", "revoke", "ghost", "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");

    let hub = FakeHub::start().await;
    hub.answer(
        "BuilderEnrol",
        refusal("FORBIDDEN", "Login or browser token only."),
    );
    let run = signed_in()
        .run(&[
            "builder",
            "enrol",
            "--name",
            "x",
            "--os",
            "linux",
            "--arch",
            "x86-64",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert_eq!(run.stdout, "");
}

#[tokio::test]
async fn cmd_024_c10_revoke_by_id_does_not_list_the_builders_first() {
    let hub = FakeHub::start().await;
    hub.answer("BuilderRevoke", data(json!({ "revokeBuilder": ID })));
    let run = signed_in()
        .run(&["builder", "revoke", ID, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, ["BuilderRevoke"]);
}
