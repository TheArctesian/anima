//! `anima notify channel` against the scripted hub.

use serde_json::json;

use super::{SMTP_PASSWORD, TOKEN, WEBHOOK_URL, channels, saved, signed_in, webhook_document};
use crate::kit::fake_hub::{FakeHub, data};

#[tokio::test]
async fn not_001_channel_list_shows_each_kind_with_its_target_and_no_secret() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ChannelList",
        data(json!({ "notificationChannels": channels() })),
    );
    let run = signed_in()
        .run(&["notify", "channel", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = |name: &str| {
        run.stdout
            .lines()
            .find(|line| line.starts_with(name))
            .expect("a row")
            .to_owned()
    };
    assert!(row("ops-webhook").contains("https://hooks.example.net"));
    assert!(row("ops-webhook").contains("ok 2026-11-06T08:00:00Z"));
    assert!(
        row("phone-ntfy").contains("https://ntfy.example.net") && row("phone-ntfy").contains("yes")
    );
    assert!(row("ops-email").contains("mail.example.net:587"));
    assert!(row("apprise-bridge").contains("never"));

    let run = signed_in()
        .run(&["notify", "channel", "list", "--json", "--hub", &hub.origin])
        .await;
    let json = run.json();
    assert_eq!(json[1]["settings"]["tokenStored"], true);
    assert_eq!(json[2]["settings"]["passwordStored"], true);
    assert_eq!(
        json[0]["settings"]["headerNames"],
        json!(["X-Anima-Source"])
    );
}

#[tokio::test]
async fn cmd_025_channel_create_reads_the_document_from_standard_input_and_never_prints_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ChannelCreate",
        saved("createNotificationChannel", "ops-hook"),
    );
    let run = signed_in()
        .run_with_input(
            &[
                "notify",
                "channel",
                "create",
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            &webhook_document(),
        )
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("ops-hook"), "{}", run.stdout);
    for secret in [WEBHOOK_URL, SMTP_PASSWORD, TOKEN] {
        run.assert_never_shows(secret);
    }
    let input = &hub.seen()[0].variables["input"];
    assert_eq!(input["name"], "ops-hook");
    assert_eq!(input["onByDefault"], true);
    assert_eq!(input["settings"]["webhook"]["url"], WEBHOOK_URL);
    assert_eq!(
        input["settings"]["webhook"]["headers"][0]["value"],
        SMTP_PASSWORD
    );
    assert!(
        input["settings"]
            .as_object()
            .is_some_and(|settings| settings.len() == 1),
        "a oneOf input names one variant only: {input}"
    );
}

#[tokio::test]
async fn cmd_025_channel_create_reads_a_file_and_a_broken_document_never_quotes_its_secret() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ChannelCreate",
        saved("createNotificationChannel", "ops-hook"),
    );
    let anima = signed_in();
    let file = anima.config_dir().with_file_name("channel.json");
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("folder");
    std::fs::write(&file, webhook_document()).expect("document");
    let path = file.to_str().expect("a path");
    let run = anima
        .run(&[
            "notify",
            "channel",
            "create",
            "--from-file",
            path,
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["createNotificationChannel"]["name"], "ops-hook");

    let broken = format!(r#"{{"name": 7, "settings": {{"email": {{"password": "{SMTP_PASSWORD}" "#);
    let run = anima
        .run_with_input(
            &[
                "notify",
                "channel",
                "create",
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            &broken,
        )
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("line 1"), "{}", run.stderr);
    run.assert_never_shows(SMTP_PASSWORD);
    assert_eq!(hub.seen().len(), 1, "the broken document was not sent");
}

#[tokio::test]
async fn not_002_channel_update_by_name_sends_the_id_and_the_document() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ChannelList",
        data(json!({ "notificationChannels": channels() })),
    )
    .answer(
        "ChannelUpdate",
        saved("updateNotificationChannel", "phone-ntfy"),
    );
    let document =
        json!({ "name": "phone-ntfy", "settings": { "ntfy": { "topic": "t", "token": "tk_x" } } });
    let run = signed_in()
        .run_with_input(
            &[
                "notify",
                "channel",
                "update",
                "phone-ntfy",
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            &document.to_string(),
        )
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let update = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "ChannelUpdate")
        .expect("an update");
    assert_eq!(
        update.variables["id"],
        "0190a000-0000-7000-8017-000000000002"
    );
    assert_eq!(
        update.variables["input"]["settings"]["ntfy"]["token"],
        "tk_x"
    );
    run.assert_never_shows("tk_x");
}

#[tokio::test]
async fn cmd_024_channel_delete_needs_a_confirmation() {
    let hub = FakeHub::start().await;
    let id = "0190a000-0000-7000-8017-000000000003";
    hub.answer(
        "ChannelList",
        data(json!({ "notificationChannels": channels() })),
    )
    .answer(
        "ChannelDelete",
        data(json!({ "deleteNotificationChannel": id })),
    );
    let refused = signed_in()
        .run(&[
            "notify",
            "channel",
            "delete",
            "ops-email",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(refused.code, 2, "{refused:?}");
    assert!(refused.stderr.contains("--yes"), "{}", refused.stderr);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "ChannelDelete")
    );

    let run = signed_in()
        .run(&[
            "notify",
            "channel",
            "delete",
            "ops-email",
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(id), "{}", run.stdout);
}

#[tokio::test]
async fn not_004_a_failed_channel_test_is_printed_and_then_exit_1_with_the_reason() {
    let hub = FakeHub::start().await;
    let failed = json!({
        "id": "0190a000-0000-7000-8018-0000000000aa", "channelId": "0190a000-0000-7000-8017-000000000002",
        "channelName": "phone-ntfy", "kind": "TEST", "at": "2026-11-13T10:00:00Z",
        "ok": false, "reason": "HTTP_STATUS", "code": 502,
    });
    hub.answer(
        "ChannelTest",
        data(json!({ "sendTestNotification": failed })),
    );
    let id = "0190a000-0000-7000-8017-000000000002";
    let run = signed_in()
        .run(&["notify", "channel", "test", id, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    let result = run
        .stdout
        .lines()
        .find(|line| line.starts_with("Result"))
        .expect("a result");
    assert!(result.contains("failed: HTTP_STATUS 502"), "{result}");
    assert!(
        run.stderr.contains("to phone-ntfy was not delivered"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("HTTP_STATUS, code 502"),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn not_004_a_delivered_channel_test_is_exit_0() {
    let hub = FakeHub::start().await;
    let delivered = json!({
        "id": "0190a000-0000-7000-8018-0000000000ab", "channelId": "0190a000-0000-7000-8017-000000000002",
        "channelName": "phone-ntfy", "kind": "TEST", "at": "2026-11-13T10:00:00Z",
        "ok": true, "reason": null, "code": null,
    });
    hub.answer(
        "ChannelTest",
        data(json!({ "sendTestNotification": delivered })),
    );
    let id = "0190a000-0000-7000-8017-000000000002";
    let run = signed_in()
        .run(&["notify", "channel", "test", id, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("ok"), "{}", run.stdout);
}

#[tokio::test]
async fn cmd_021_channel_test_for_a_missing_channel_is_exit_3() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ChannelList",
        data(json!({ "notificationChannels": channels() })),
    );
    let run = signed_in()
        .run(&[
            "notify",
            "channel",
            "test",
            "no-such",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(run.error()["code"], "NOT_FOUND");
}

#[tokio::test]
async fn cmd_021_channel_create_refused_for_a_target_is_a_validation_error_with_its_fields() {
    let hub = FakeHub::start().await;
    let body = json!({ "data": null, "errors": [{
        "message": "Check the channel.",
        "extensions": { "code": "VALIDATION",
                        "fields": [{ "field": "settings.webhook.url", "message": "Use https." }] },
    }]});
    hub.answer("ChannelCreate", body);
    let run = signed_in()
        .run_with_input(
            &[
                "notify",
                "channel",
                "create",
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            &webhook_document(),
        )
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.stderr.contains("settings.webhook.url: Use https."),
        "{}",
        run.stderr
    );
    run.assert_never_shows(WEBHOOK_URL);
}
