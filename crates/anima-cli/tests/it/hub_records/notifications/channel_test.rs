//! `anima notify channel test` against the scripted hub (NOT-004, CMD-021).

use serde_json::json;

use super::{channels, signed_in};
use crate::kit::fake_hub::{FakeHub, data};

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
