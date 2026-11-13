//! `anima notify settings` and `deliveries` against the scripted hub.

use serde_json::{Value, json};

use super::{channels, fixtures, signed_in};
use crate::kit::fake_hub::{FakeHub, data};

#[tokio::test]
async fn not_006_settings_set_keeps_what_it_is_not_told_to_change() {
    let hub = FakeHub::start().await;
    let settings = fixtures()["notificationSettings"].clone();
    let mut changed = settings.clone();
    changed["certificateDays"] = json!([30, 7]);
    hub.answer(
        "SettingsGet",
        data(json!({ "notificationSettings": settings })),
    )
    .answer(
        "SettingsSet",
        data(json!({ "setNotificationSettings": changed })),
    );
    let get = signed_in()
        .run(&["notify", "settings", "get", "--hub", &hub.origin])
        .await;
    assert_eq!(get.code, 0, "{get:?}");
    let days = get
        .stdout
        .lines()
        .find(|line| line.starts_with("Certificate days"))
        .expect("days");
    assert!(days.ends_with("21, 14, 7"), "{days}");

    let run = signed_in()
        .run(&[
            "notify",
            "settings",
            "set",
            "--certificate-days",
            "30,7",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let set = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "SettingsSet")
        .expect("a set");
    assert_eq!(
        set.variables["input"],
        json!({ "certificateDays": [30, 7], "template": settings["template"] })
    );

    let nothing = signed_in()
        .run(&["notify", "settings", "set", "--hub", &hub.origin])
        .await;
    assert_eq!(nothing.code, 2, "{nothing:?}");
}

#[tokio::test]
async fn not_004_deliveries_show_the_reason_and_filter_by_channel_name() {
    let hub = FakeHub::start().await;
    let nodes = fixtures()["deliveries"].clone();
    let edges: Vec<Value> = nodes
        .as_array()
        .expect("deliveries")
        .iter()
        .map(|node| json!({ "cursor": "c", "node": node }))
        .collect();
    hub.answer(
        "ChannelList",
        data(json!({ "notificationChannels": channels() })),
    )
    .answer(
        "DeliveryList",
        data(json!({ "deliveries": { "edges": edges,
                "pageInfo": { "hasNextPage": false, "endCursor": null } } })),
    );
    let run = signed_in()
        .run(&[
            "notify",
            "deliveries",
            "--channel",
            "phone-ntfy",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let failed = run
        .stdout
        .lines()
        .find(|line| line.contains("TARGET_NOT_PERMITTED"))
        .expect("a failure");
    assert!(
        failed.contains("phone-ntfy") && failed.contains("failed"),
        "{failed}"
    );
    let delivery = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "DeliveryList")
        .expect("a list");
    assert_eq!(
        delivery.variables["channelId"],
        "0190a000-0000-7000-8017-000000000002"
    );
    assert_eq!(delivery.variables["first"], 50);
}
