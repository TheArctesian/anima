//! The reminder rules and their firings through the hub API (K-E6 Section 6; REM-004, REM-005,
//! REM-007).

mod firings;

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, error, set_operator, start};

const RULE: &str = "id name message trigger unless cooldownMinutes dailyLimit channelIds enabled \
                    revision lastFiringAt firedToday";
const FIRING: &str = "id ruleId subject text state reason snoozedUntil decidedAt";

async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

fn daily() -> Value {
    json!({ "name": " Stretch ", "message": "Stand up and stretch",
            "trigger": { "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" } })
}

async fn create(hub: &Hub, token: &str, input: Value) -> Value {
    let query = format!(
        "mutation($i: ReminderRuleInput!) {{ createReminderRule(input: $i) {{ {RULE} }} }}"
    );
    hub.post(Some(token), &query, json!({ "i": input })).await
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_004_a_rule_is_made_with_the_default_cooldown_and_limit_and_is_listed(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let made = create(&hub, &token, daily()).await;
    let rule = &made["data"]["createReminderRule"];
    assert_eq!(rule["name"], "Stretch", "{made}");
    assert_eq!(
        (&rule["cooldownMinutes"], &rule["dailyLimit"]),
        (&json!(60), &json!(3))
    );
    assert_eq!(
        (&rule["enabled"], &rule["revision"]),
        (&json!(true), &json!(1))
    );
    assert_eq!(
        rule["trigger"],
        json!({ "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" })
    );
    assert!(rule["lastFiringAt"].is_null());
    let listed = hub
        .post(
            Some(&token),
            &format!("{{ reminderRules {{ {RULE} }} }}"),
            json!({}),
        )
        .await;
    assert_eq!(listed["data"]["reminderRules"][0]["id"], rule["id"]);
    let audit: i64 = sqlx::query_scalar(
        "select count(*) from events.audit_log where target_type = 'reminder_rule' \
         and action = 'create' and change::text not like '%Stand up%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, 1, "one audit entry, without the message of the rule");
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_001_a_rule_that_is_not_valid_names_each_field_to_fix(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let mut bad = daily();
    bad["trigger"] = json!({ "type": "time", "at": "09:00", "zone": "Mars/Olympus" });
    let refused = create(&hub, &token, bad).await;
    assert_eq!(error(&refused).1, "VALIDATION");
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "trigger.zone"
    );

    let mut limits = daily();
    limits["name"] = json!("");
    limits["cooldownMinutes"] = json!(0);
    limits["dailyLimit"] = json!(101);
    let refused = create(&hub, &token, limits).await;
    let fields: Vec<&str> = refused["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["name", "cooldownMinutes", "dailyLimit"]);

    let mut nowhere = daily();
    nowhere["trigger"] = json!({ "type": "place", "on": "arrive",
                                 "place": "0190a000-0000-7000-9200-000000000001" });
    let refused = create(&hub, &token, nowhere).await;
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "trigger.place"
    );
    let mut channel = daily();
    channel["channelIds"] = json!(["0190a000-0000-7000-9200-000000000002"]);
    let refused = create(&hub, &token, channel).await;
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "channelIds"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_005_each_change_of_a_rule_adds_one_to_its_revision(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let made = create(&hub, &token, daily()).await;
    let id = made["data"]["createReminderRule"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut changed = daily();
    changed["dailyLimit"] = json!(1);
    let update = format!(
        "mutation($id: ID!, $i: ReminderRuleInput!) {{ updateReminderRule(id: $id, input: $i) \
         {{ {RULE} }} }}"
    );
    let updated = hub
        .post(Some(&token), &update, json!({ "id": id, "i": changed }))
        .await;
    let rule = &updated["data"]["updateReminderRule"];
    assert_eq!(
        (&rule["revision"], &rule["dailyLimit"]),
        (&json!(2), &json!(1)),
        "{updated}"
    );

    let switch = "mutation($id: ID!, $on: Boolean!) { setReminderRuleEnabled(id: $id, enabled: $on) \
                  { enabled revision } }";
    let off = hub
        .post(Some(&token), switch, json!({ "id": id, "on": false }))
        .await;
    assert_eq!(
        off["data"]["setReminderRuleEnabled"],
        json!({ "enabled": false, "revision": 3 })
    );
    let again = hub
        .post(Some(&token), switch, json!({ "id": id, "on": false }))
        .await;
    assert_eq!(
        again["data"]["setReminderRuleEnabled"]["revision"], 3,
        "no change, no revision"
    );

    let remove = "mutation($id: ID!) { removeReminderRule(id: $id) }";
    let removed = hub.post(Some(&token), remove, json!({ "id": id })).await;
    assert_eq!(removed["data"]["removeReminderRule"], id.as_str());
    let gone = hub.post(Some(&token), remove, json!({ "id": id })).await;
    assert_eq!(error(&gone).1, "NOT_FOUND");
}
