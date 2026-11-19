//! A test of a rule, and the answers of the operator to a firing (REM-007).

use serde_json::json;
use sqlx::PgPool;

use super::{FIRING, create, daily, signed_in};
use crate::hub::error;

#[sqlx::test(migrations = "./migrations")]
async fn rem_007_a_test_makes_a_firing_and_only_a_sent_firing_is_snoozed_or_acknowledged(
    pool: PgPool,
) {
    let (hub, token) = signed_in(&pool).await;
    let made = create(&hub, &token, daily()).await;
    let id = made["data"]["createReminderRule"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let test = format!("mutation($id: ID!) {{ testReminderRule(id: $id) {{ {FIRING} }} }}");
    let tested = hub.post(Some(&token), &test, json!({ "id": id })).await;
    let firing = &tested["data"]["testReminderRule"];
    assert_eq!(
        (&firing["state"], &firing["text"]),
        (&json!("PENDING"), &json!("Stand up and stretch"))
    );
    assert!(
        firing["subject"].as_str().unwrap().starts_with("test:"),
        "{tested}"
    );
    let firing_id = firing["id"].as_str().unwrap().to_owned();

    let snooze = format!(
        "mutation($f: ID!, $m: Int!) {{ snoozeReminder(firing: $f, minutes: $m) {{ {FIRING} }} }}"
    );
    let early = hub
        .post(Some(&token), &snooze, json!({ "f": firing_id, "m": 10 }))
        .await;
    assert_eq!(error(&early).1, "CONFLICT", "it was not sent yet");

    sqlx::query("update reminders.firing set state = 'sent', sent_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    let long = hub
        .post(
            Some(&token),
            &snooze,
            json!({ "f": firing_id, "m": 10_081 }),
        )
        .await;
    assert_eq!(error(&long).1, "VALIDATION");
    let snoozed = hub
        .post(Some(&token), &snooze, json!({ "f": firing_id, "m": 10 }))
        .await;
    let state = &snoozed["data"]["snoozeReminder"];
    assert_eq!(state["state"], "SNOOZED", "{snoozed}");
    assert_eq!(state["snoozedUntil"], "2030-05-05T12:10:00+00:00");

    let ack = format!("mutation($f: ID!) {{ acknowledgeReminder(firing: $f) {{ {FIRING} }} }}");
    let done = hub
        .post(Some(&token), &ack, json!({ "f": firing_id }))
        .await;
    assert_eq!(done["data"]["acknowledgeReminder"]["state"], "ACKNOWLEDGED");
    let twice = hub
        .post(Some(&token), &ack, json!({ "f": firing_id }))
        .await;
    assert_eq!(error(&twice).1, "CONFLICT");

    let list = format!(
        "query($r: ID) {{ reminderFirings(rule: $r, state: ACKNOWLEDGED) {{ {FIRING} }} }}"
    );
    let listed = hub.post(Some(&token), &list, json!({ "r": id })).await;
    assert_eq!(
        listed["data"]["reminderFirings"].as_array().map(Vec::len),
        Some(1)
    );
    let rules = hub
        .post(
            Some(&token),
            "{ reminderRules { firedToday lastFiringAt } }",
            json!({}),
        )
        .await;
    assert_eq!(rules["data"]["reminderRules"][0]["firedToday"], 1);
}
