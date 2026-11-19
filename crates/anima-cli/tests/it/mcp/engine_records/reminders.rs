//! The reminder tools of the `records` set against the scripted hub (MCP-002, MCP-017; K-E6
//! Section 6).

use serde_json::{Value, json};

use crate::kit::fake_hub::data;
use crate::mcp::engine::{sent, structured, writing};
use crate::mcp::session::{Desk, LOGIN_TOKEN, hub};

const ID: &str = "0190a000-0000-7000-9400-000000000001";

fn rule() -> Value {
    json!({ "id": ID, "name": "Stretch", "message": "Stand up",
            "trigger": { "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" },
            "unless": null, "cooldownMinutes": 60, "dailyLimit": 3, "channelIds": [],
            "enabled": true, "revision": 1, "createdAt": "2026-11-19T10:00:00+00:00",
            "updatedAt": "2026-11-19T10:00:00+00:00", "lastFiringAt": null, "firedToday": 0 })
}

fn firing(state: &str) -> Value {
    json!({ "id": "f1", "ruleId": ID, "subject": "test:1", "text": "Stand up", "state": state,
            "reason": null, "dueAt": "2026-11-19T08:00:00+00:00", "sentAt": null,
            "snoozedUntil": null, "decidedAt": null })
}

#[tokio::test]
async fn mcp_002_the_reminder_tools_that_change_are_off_until_writing_is_allowed() {
    let hub = hub().await;
    let mut reading = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let names = reading.tool_names().await;
    for read in [
        "anima_reminders_list",
        "anima_reminder_get",
        "anima_reminder_firings",
    ] {
        assert!(names.iter().any(|name| name == read), "{read}");
    }
    for changing in [
        "anima_reminder_add",
        "anima_reminder_set",
        "anima_reminder_remove",
        "anima_reminder_set_enabled",
        "anima_reminder_test",
        "anima_reminder_snooze",
        "anima_reminder_ack",
    ] {
        assert!(!names.iter().any(|name| name == changing), "{changing}");
    }
    reading.finish().await;
}

#[tokio::test]
async fn mcp_017_the_reminder_tools_add_list_test_and_answer_a_firing() {
    let hub = hub().await;
    hub.answer("ReminderAdd", data(json!({ "createReminderRule": rule() })));
    hub.answer("ReminderList", data(json!({ "reminderRules": [rule()] })));
    hub.answer(
        "ReminderTest",
        data(json!({ "testReminderRule": firing("PENDING") })),
    );
    hub.answer(
        "ReminderFirings",
        data(json!({ "reminderFirings": [firing("SENT")] })),
    );
    hub.answer(
        "ReminderSnooze",
        data(json!({ "snoozeReminder": firing("SNOOZED") })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;

    let trigger = json!({ "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" });
    let arguments = json!({ "name": "Stretch", "message": "Stand up", "trigger": trigger });
    let added = session.call("anima_reminder_add", arguments).await;
    assert_eq!(structured(&added)["rule"]["id"], ID);
    let input = &sent(&hub, "ReminderAdd")[0]["input"];
    assert_eq!(input["trigger"], trigger);
    assert_eq!(
        (&input["cooldownMinutes"], &input["dailyLimit"]),
        (&json!(60), &json!(3))
    );

    let listed = session.call("anima_reminders_list", json!({})).await;
    assert_eq!(structured(&listed)["rules"][0]["name"], "Stretch");
    let tested = session
        .call("anima_reminder_test", json!({ "id": ID }))
        .await;
    assert_eq!(structured(&tested)["firing"]["state"], "PENDING");
    let firings = session
        .call(
            "anima_reminder_firings",
            json!({ "rule": ID, "state": "SENT" }),
        )
        .await;
    assert_eq!(structured(&firings)["firings"][0]["state"], "SENT");
    assert_eq!(sent(&hub, "ReminderFirings")[0]["state"], "SENT");
    let snoozed = session
        .call(
            "anima_reminder_snooze",
            json!({ "firing": "f1", "minutes": 15 }),
        )
        .await;
    assert_eq!(structured(&snoozed)["firing"]["state"], "SNOOZED");
    assert_eq!(
        sent(&hub, "ReminderSnooze")[0],
        json!({ "firing": "f1", "minutes": 15 })
    );
    session.finish().await;
}
