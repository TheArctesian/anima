//! `anima reminder` against the scripted hub (K-E6 Section 6; REM-004, REM-005, REM-007).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const ID: &str = "0190a000-0000-7000-9400-000000000001";
const FIRING: &str = "0190a000-0000-7000-9400-000000000002";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-reminder-tests")
}

fn rule() -> Value {
    json!({ "id": ID, "name": "Stretch", "message": "Stand up",
            "trigger": { "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" },
            "unless": null, "cooldownMinutes": 60, "dailyLimit": 3, "channelIds": [],
            "enabled": true, "revision": 1, "createdAt": "2026-11-19T10:00:00+00:00",
            "updatedAt": "2026-11-19T10:00:00+00:00", "lastFiringAt": null, "firedToday": 0 })
}

fn firing(state: &str) -> Value {
    json!({ "id": FIRING, "ruleId": ID, "subject": "time:2026-11-19T08:00:00Z",
            "text": "Stand up", "state": state, "reason": null,
            "dueAt": "2026-11-19T08:00:00+00:00", "sentAt": null, "snoozedUntil": null,
            "decidedAt": null })
}

#[tokio::test]
async fn rem_004_reminder_add_sends_the_rule_with_the_default_limits() {
    let hub = FakeHub::start().await;
    hub.answer("ReminderAdd", data(json!({ "createReminderRule": rule() })));
    let trigger = r#"{"type":"time","at":"09:00","zone":"Europe/Amsterdam"}"#;
    let run = signed_in()
        .run(&[
            "reminder",
            "add",
            "--name",
            "Stretch",
            "--message",
            "Stand up",
            "--trigger",
            trigger,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("Stretch") && run.stdout.contains(ID),
        "{}",
        run.stdout
    );
    let sent = &hub.seen()[0].variables["input"];
    assert_eq!(
        sent["trigger"],
        json!({ "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" })
    );
    assert_eq!(
        (
            &sent["cooldownMinutes"],
            &sent["dailyLimit"],
            &sent["enabled"]
        ),
        (&json!(60), &json!(3), &json!(true))
    );
}

#[tokio::test]
async fn rem_001_reminder_add_without_a_trigger_or_with_bad_json_asks_the_hub_nothing() {
    let hub = FakeHub::start().await;
    let missing = signed_in()
        .run(&[
            "reminder",
            "add",
            "--name",
            "Stretch",
            "--message",
            "Stand up",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(missing.code, 2, "{missing:?}");
    assert!(missing.stderr.contains("--trigger"), "{}", missing.stderr);
    let bad = signed_in()
        .run(&[
            "reminder",
            "add",
            "--name",
            "Stretch",
            "--message",
            "Stand up",
            "--trigger",
            "at nine",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(bad.code, 2, "{bad:?}");
    assert!(!bad.stderr.contains("at nine"), "{}", bad.stderr);
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn rem_005_reminder_set_changes_only_what_is_given() {
    let hub = FakeHub::start().await;
    hub.answer("ReminderShow", data(json!({ "reminderRule": rule() })));
    hub.answer("ReminderSet", data(json!({ "updateReminderRule": rule() })));
    let run = signed_in()
        .run(&[
            "reminder",
            "set",
            ID,
            "--daily-limit",
            "1",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let seen = hub.seen();
    let sent = &seen[1].variables;
    assert_eq!(sent["id"], ID);
    assert_eq!(sent["input"]["dailyLimit"], 1);
    assert_eq!(
        (&sent["input"]["name"], &sent["input"]["cooldownMinutes"]),
        (&json!("Stretch"), &json!(60))
    );
    assert_eq!(sent["input"]["trigger"]["at"], "09:00");
    assert!(sent["input"].get("revision").is_none(), "{sent}");
}

#[tokio::test]
async fn rem_001_reminder_list_shows_each_rule_with_its_trigger() {
    let hub = FakeHub::start().await;
    hub.answer("ReminderList", data(json!({ "reminderRules": [rule()] })));
    let run = signed_in()
        .run(&["reminder", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|l| l.starts_with("Stretch"))
        .expect("the row");
    assert!(
        row.contains("at 09:00 (Europe/Amsterdam)") && row.contains("yes"),
        "{row}"
    );
}

#[tokio::test]
async fn rem_007_reminder_snooze_and_ack_send_the_firing() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ReminderSnooze",
        data(json!({ "snoozeReminder": firing("SNOOZED") })),
    );
    hub.answer(
        "ReminderAck",
        data(json!({ "acknowledgeReminder": firing("ACKNOWLEDGED") })),
    );
    let snoozed = signed_in()
        .run(&["reminder", "snooze", FIRING, "30", "--hub", &hub.origin])
        .await;
    assert_eq!(snoozed.code, 0, "{snoozed:?}");
    assert!(snoozed.stdout.contains("SNOOZED"), "{}", snoozed.stdout);
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "firing": FIRING, "minutes": 30 })
    );
    let acked = signed_in()
        .run(&["reminder", "ack", FIRING, "--hub", &hub.origin])
        .await;
    assert_eq!(acked.code, 0, "{acked:?}");
    assert_eq!(hub.seen()[1].variables, json!({ "firing": FIRING }));
    let long = signed_in()
        .run(&["reminder", "snooze", FIRING, "10081", "--hub", &hub.origin])
        .await;
    assert_eq!(long.code, 2, "more than one week is refused before the hub");
}

#[tokio::test]
async fn rem_005_reminder_disable_and_remove_name_the_rule() {
    let hub = FakeHub::start().await;
    let mut off = rule();
    off["enabled"] = json!(false);
    hub.answer(
        "ReminderEnable",
        data(json!({ "setReminderRuleEnabled": off })),
    );
    hub.answer("ReminderRemove", data(json!({ "removeReminderRule": ID })));
    let disabled = signed_in()
        .run(&["reminder", "disable", ID, "--hub", &hub.origin])
        .await;
    assert_eq!(disabled.code, 0, "{disabled:?}");
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "id": ID, "enabled": false })
    );
    let removed = signed_in()
        .run(&["reminder", "remove", ID, "--hub", &hub.origin])
        .await;
    assert_eq!(removed.code, 0, "{removed:?}");
    assert!(removed.stdout.contains(ID), "{}", removed.stdout);
}
