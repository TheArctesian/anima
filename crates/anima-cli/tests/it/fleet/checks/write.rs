//! `check create|update|delete|pause|resume|defaults|notifications` (CMD-024, CMD-025).

use serde_json::{Value, json};

use super::{CHECK, DEVICE, operations, signed_in, variables};
use crate::kit::fake_hub::{FakeHub, data, refusal};

fn written(field: &str, paused: bool) -> Value {
    data(json!({ field: {
        "id": CHECK, "name": "SSH port", "type": "TCP", "status": "UP", "paused": paused,
    }}))
}

#[tokio::test]
async fn cmd_025_check_create_reads_the_config_from_stdin_and_never_prints_a_secret() {
    let hub = FakeHub::start().await;
    hub.answer("CheckCreate", written("createCheck", false));
    let config = r#"{"snmp": {"oid": "1.3.6.1", "community": "s3cret-community"}}"#;
    let run = signed_in()
        .run_with_input(
            &[
                "check",
                "create",
                "--device",
                DEVICE,
                "--name",
                "SSH port",
                "--from-file",
                "-",
                "--interval-seconds",
                "30",
                "--hub",
                &hub.origin,
            ],
            config,
        )
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("SSH port") && run.stdout.contains(CHECK));
    run.assert_never_shows("s3cret-community");
    let input = variables(&hub, "CheckCreate")["input"].clone();
    assert_eq!(input["deviceId"], DEVICE);
    assert_eq!(input["intervalSeconds"], 30);
    assert_eq!(input["config"]["snmp"]["community"], "s3cret-community");
    assert_eq!(input["config"]["snmp"]["oid"], "1.3.6.1");
}

#[tokio::test]
async fn cmd_025_check_create_refuses_a_config_that_is_not_one_kind_before_calling_the_hub() {
    let hub = FakeHub::start().await;
    for config in [
        r#"{"tcp": {"port": 22}, "icmp": {}}"#,
        r#"{"ftp": {}}"#,
        "nope",
    ] {
        let run = signed_in()
            .run_with_input(
                &[
                    "check",
                    "create",
                    "--device",
                    DEVICE,
                    "--name",
                    "x",
                    "--from-file",
                    "-",
                    "--hub",
                    &hub.origin,
                ],
                config,
            )
            .await;
        assert_eq!(run.code, 2, "{config}: {run:?}");
    }
    assert_eq!(operations(&hub), Vec::<String>::new());
}

#[tokio::test]
async fn cmd_021_a_refused_check_create_exits_5() {
    let hub = FakeHub::start().await;
    hub.answer("CheckCreate", refusal("FORBIDDEN", "Read-only token."));
    let run = signed_in()
        .run_with_input(
            &[
                "check",
                "create",
                "--device",
                DEVICE,
                "--name",
                "x",
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            r#"{"tcp": {"port": 22}}"#,
        )
        .await;
    assert_eq!(run.code, 5, "{run:?}");
}

#[tokio::test]
async fn check_update_sends_only_the_settings_named() {
    let hub = FakeHub::start().await;
    hub.answer("CheckUpdate", written("updateCheck", false));
    let run = signed_in()
        .run(&[
            "check",
            "update",
            CHECK,
            "--retries",
            "5",
            "--invert",
            "true",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sent = variables(&hub, "CheckUpdate");
    assert_eq!(sent["id"], CHECK);
    assert_eq!(sent["input"], json!({ "retries": 5, "invert": true }));

    let run = signed_in()
        .run(&["check", "update", CHECK, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
}

#[tokio::test]
async fn check_update_no_parent_sends_an_explicit_null_parent_with_the_other_settings() {
    let hub = FakeHub::start().await;
    hub.answer("CheckUpdateUngrouped", written("updateCheck", false));
    let run = signed_in()
        .run(&[
            "check",
            "update",
            CHECK,
            "--no-parent",
            "--retries",
            "5",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sent = variables(&hub, "CheckUpdateUngrouped");
    assert_eq!(sent, json!({ "id": CHECK, "retries": 5 }));
    let document = include_str!("../../../../src/checks/write/update_ungrouped.graphql");
    assert!(
        document.contains("parentId: null"),
        "the document clears the parent"
    );

    let run = signed_in()
        .run(&[
            "check",
            "update",
            CHECK,
            "--no-parent",
            "--parent",
            "x",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "--no-parent and --parent conflict: {run:?}");
}

#[tokio::test]
async fn cmd_024_check_delete_needs_a_confirmation() {
    let hub = FakeHub::start().await;
    hub.answer("CheckDelete", data(json!({ "deleteCheck": CHECK })));
    let run = signed_in()
        .run(&["check", "delete", CHECK, "--no-input", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--yes"), "{}", run.stderr);
    assert_eq!(operations(&hub), Vec::<String>::new());

    let run = signed_in()
        .run(&["check", "delete", CHECK, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(CHECK), "{}", run.stdout);
    assert_eq!(variables(&hub, "CheckDelete")["id"], CHECK);
}

#[tokio::test]
async fn check_pause_and_resume_set_the_flag() {
    let hub = FakeHub::start().await;
    hub.answer("CheckSetPaused", written("setCheckPaused", true))
        .answer("CheckSetPaused", written("setCheckPaused", false));
    let run = signed_in()
        .run(&["check", "pause", CHECK, "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("PAUSED"), "{run:?}");
    let run = signed_in()
        .run(&["check", "resume", CHECK, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sent: Vec<Value> = hub
        .seen()
        .iter()
        .map(|seen| seen.variables["paused"].clone())
        .collect();
    assert_eq!(sent, [json!(true), json!(false)]);
}

#[tokio::test]
async fn check_defaults_set_keeps_the_values_not_named() {
    let hub = FakeHub::start().await;
    let defaults = |every: i64| {
        json!({ "intervalSeconds": every, "retries": 3, "retryIntervalSeconds": 20,
                "degradedThresholdMs": 1000 })
    };
    hub.answer(
        "CheckDefaultsGet",
        data(json!({ "checkDefaults": defaults(60) })),
    )
    .answer(
        "CheckDefaultsSet",
        data(json!({ "setCheckDefaults": defaults(120) })),
    );
    let run = signed_in()
        .run(&["check", "defaults", "get", "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("60 s"), "{run:?}");
    let run = signed_in()
        .run(&[
            "check",
            "defaults",
            "set",
            "--interval-seconds",
            "120",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("120 s"), "{}", run.stdout);
    assert_eq!(variables(&hub, "CheckDefaultsSet")["input"], defaults(120));
}

#[tokio::test]
async fn check_notifications_set_sends_the_channels_and_the_repeat() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CheckNotificationsSet",
        data(json!({ "setCheckNotifications": { "resendEveryBeats": 10,
            "channels": [{ "id": "c1", "name": "ntfy", "kind": "NTFY" }] } })),
    );
    let run = signed_in()
        .run(&[
            "check",
            "notifications",
            "set",
            CHECK,
            "--channel",
            "c1",
            "--resend-every-beats",
            "10",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("ntfy") && run.stdout.contains("10 Down"),
        "{}",
        run.stdout
    );
    assert_eq!(
        variables(&hub, "CheckNotificationsSet"),
        json!({ "checkId": CHECK, "input": { "channelIds": ["c1"], "resendEveryBeats": 10 } })
    );
}
