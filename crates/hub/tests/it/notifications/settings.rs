//! The notification settings: certificate thresholds and the default template (NOT-006,
//! NOT-007, D-21, D-22), through the API.

use anima_hub::fleet::events::Source;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CLIENT_ADDR, OPERATOR_ID, create_query, data, error_json, run, schema_for, validation_fields,
    webhook,
};
use crate::hub::{Hub, error, set_operator, start};

const READ: &str =
    "{ notificationSettings { certificateDays placeholders template { title body } } }";
const SET: &str = "mutation($input: NotificationSettingsInput!) { \
    setNotificationSettings(input: $input) { certificateDays placeholders template { title body } } }";

fn input(days: &Value, title: &str, body: &str) -> Value {
    json!({ "input": { "certificateDays": days, "template": { "title": title, "body": body } } })
}

async fn stored(pool: &PgPool) -> (Vec<i32>, String, String) {
    sqlx::query_as(
        "SELECT certificate_days, title_template, body_template FROM notifications.settings",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn not_006_the_defaults_read_back_and_save_again_unchanged(pool: PgPool) {
    let schema = schema_for(&pool);
    let read = data(&schema, READ, json!({})).await["notificationSettings"].clone();
    assert_eq!(read["certificateDays"], json!([21, 14, 7]));
    assert_eq!(
        read["placeholders"],
        json!([
            "{device}",
            "{check}",
            "{status}",
            "{previous}",
            "{message}",
            "{at}",
            "{days}"
        ])
    );
    // D-21: the seeded default template is itself valid.
    let again = json!({ "input": { "certificateDays": read["certificateDays"], "template": read["template"] } });
    let saved = data(&schema, SET, again).await;
    assert_eq!(saved["setNotificationSettings"], read);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_006_days_are_kept_largest_first_without_repeats(pool: PgPool) {
    let schema = schema_for(&pool);
    let saved = data(
        &schema,
        SET,
        input(&json!([7, 30, 14, 14, 30]), "{check}", "{days} days"),
    )
    .await;
    let saved = &saved["setNotificationSettings"];
    assert_eq!(saved["certificateDays"], json!([30, 14, 7]));
    assert_eq!(
        saved["template"],
        json!({ "title": "{check}", "body": "{days} days" })
    );
    assert_eq!(
        stored(&pool).await,
        (
            vec![30, 14, 7],
            "{check}".to_owned(),
            "{days} days".to_owned()
        )
    );
    let read = data(&schema, READ, json!({})).await;
    assert_eq!(read["notificationSettings"], *saved);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_006_days_outside_the_bounds_are_refused_and_nothing_changes(pool: PgPool) {
    let schema = schema_for(&pool);
    let before = stored(&pool).await;
    let six = json!([1, 2, 3, 4, 5, 6]);
    for days in [
        json!([]),
        json!([0]),
        json!([366]),
        json!([-7]),
        six,
        json!([21, 400]),
    ] {
        let response = run(&schema, SET, input(&days, "t", "b")).await;
        assert_eq!(validation_fields(&response), ["certificateDays"], "{days}");
    }
    let edge = data(&schema, SET, input(&json!([365, 1]), "t", "b")).await;
    assert_eq!(
        edge["setNotificationSettings"]["certificateDays"],
        json!([365, 1])
    );
    let five = data(&schema, SET, input(&json!([5, 4, 3, 2, 1, 1]), "t", "b")).await;
    assert_eq!(
        five["setNotificationSettings"]["certificateDays"],
        json!([5, 4, 3, 2, 1])
    );
    assert_ne!(stored(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_007_an_unknown_placeholder_or_stray_brace_names_the_field(pool: PgPool) {
    let schema = schema_for(&pool);
    let before = stored(&pool).await;
    let cases = [
        ("{host}", "ok", vec!["template.title"]),
        ("ok", "{check", vec!["template.body"]),
        ("}", "{}", vec!["template.title", "template.body"]),
        (
            &"t".repeat(501),
            &"b".repeat(4001),
            vec!["template.title", "template.body"],
        ),
    ];
    for (title, body, fields) in cases {
        let response = run(&schema, SET, input(&json!([7]), title, body)).await;
        assert_eq!(validation_fields(&response), fields);
    }
    let both = run(&schema, SET, input(&json!([]), "{x}", "ok")).await;
    assert_eq!(
        validation_fields(&both),
        ["certificateDays", "template.title"]
    );
    assert_eq!(stored(&pool).await, before);
    let literal = data(
        &schema,
        SET,
        input(&json!([7]), "{{device}}", &"b".repeat(4000)),
    )
    .await;
    assert_eq!(
        literal["setNotificationSettings"]["template"]["title"],
        "{{device}}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_21_a_channel_template_is_checked_for_placeholders_too(pool: PgPool) {
    let schema = schema_for(&pool);
    let mut channel = webhook("Ops");
    channel["template"] = json!({ "title": "{nope}", "body": "{check} {" });
    let response = run(&schema, &create_query(), json!({ "input": channel })).await;
    assert_eq!(
        validation_fields(&response),
        ["template.title", "template.body"]
    );
    let error = error_json(&response);
    assert!(
        error["extensions"]["fields"][0]["message"]
            .as_str()
            .unwrap()
            .contains("{nope}")
    );
    channel["template"] = json!({ "title": "{{{device}}}", "body": "{check}" });
    data(&schema, &create_query(), json!({ "input": channel })).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_change_writes_one_audit_entry_on_the_setting_and_one_event(pool: PgPool) {
    let schema = schema_for(&pool);
    data(
        &schema,
        SET,
        input(&json!([3, 9]), "T {check}", "B {message}"),
    )
    .await;
    run(&schema, SET, input(&json!([0]), "t", "b")).await;
    let audits: Vec<Value> = sqlx::query_scalar("SELECT to_jsonb(t) FROM events.audit_log t")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    let audit = &audits[0];
    assert_eq!(audit["action"], "update");
    assert_eq!(audit["target_type"], "setting");
    assert_eq!(audit["target_id"], "notifications.settings");
    assert_eq!(audit["operator_id"], OPERATOR_ID);
    assert_eq!(audit["client_addr"], CLIENT_ADDR);
    assert_eq!(
        audit["change"],
        json!({ "certificateDays": [9, 3], "template": { "title": "T {check}", "body": "B {message}" } })
    );
    let events: Vec<Value> = sqlx::query_scalar("SELECT to_jsonb(t) FROM events.event_log t")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["source"], Source::Notifications.as_str());
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_a_read_key_reads_the_settings_but_cannot_change_them(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let key = super::guard::api_key(&pool, 7, "read").await;
    let writer = super::guard::api_key(&pool, 8, "write").await;
    let read = hub.post(Some(&key), READ, json!({})).await;
    assert_eq!(
        read["data"]["notificationSettings"]["certificateDays"],
        json!([21, 14, 7])
    );
    let refused = hub
        .post(Some(&key), SET, input(&json!([7]), "t", "b"))
        .await;
    assert_eq!(
        error(&refused),
        (
            "This token may only read.".to_owned(),
            "FORBIDDEN".to_owned()
        )
    );
    assert_eq!(stored(&pool).await.0, vec![21, 14, 7]);
    // R-100: a write key may.
    let saved = hub
        .post(Some(&writer), SET, input(&json!([7]), "t", "b"))
        .await;
    assert_eq!(saved["errors"], Value::Null, "{saved}");
    let none = hub.post(None, READ, json!({})).await;
    assert_eq!(error(&none).1, "UNAUTHENTICATED");
    assert_eq!(stored(&pool).await.0, vec![7]);
}
