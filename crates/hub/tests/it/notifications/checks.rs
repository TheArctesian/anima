//! `setCheckNotifications`: the channels a check notifies and its resend setting (NOT-003,
//! NOT-005, D-22, API-005), through the API.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{OPERATOR_ID, create, data, run, schema_for, validation_fields, webhook};
use crate::hub::{Hub, error, set_operator, start};

const PING: &str = "0190a000-0000-7000-8003-000000000001";
const SSH_PORT: &str = "0190a000-0000-7000-8003-000000000002";
const NO_CHECK: &str = "0190a000-0000-7000-8003-0000000000ff";
const NO_CHANNEL: &str = "0190a000-0000-7000-8fff-0000000000ff";

const SET: &str = "mutation($check: ID!, $input: CheckNotificationsInput!) { \
    setCheckNotifications(checkId: $check, input: $input) { \
        resendEveryBeats channels { id name kind } } }";

fn input(channels: &Value, resend: Option<i64>) -> Value {
    match resend {
        Some(beats) => json!({ "channelIds": channels, "resendEveryBeats": beats }),
        None => json!({ "channelIds": channels }),
    }
}

async fn set(
    schema: &super::harness::TestSchema,
    check: &str,
    input: Value,
) -> async_graphql::Response {
    run(schema, SET, json!({ "check": check, "input": input })).await
}

async fn links(pool: &PgPool, check: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT channel_id::text FROM notifications.check_channels \
         WHERE check_id = $1::text::uuid ORDER BY 1",
    )
    .bind(check)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn resend(pool: &PgPool, check: &str) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT resend_every_beats FROM notifications.check_settings \
         WHERE check_id = $1::text::uuid",
    )
    .bind(check)
    .fetch_optional(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn not_003_the_channel_set_is_replaced_and_the_answer_lists_it(pool: PgPool) {
    let schema = schema_for(&pool);
    let a = create(&schema, webhook("Alpha")).await["id"].clone();
    let b = create(&schema, webhook("Bravo")).await["id"].clone();
    let first = set(&schema, PING, input(&json!([b, a, b]), Some(5))).await;
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let first = first.data.into_json().unwrap()["setCheckNotifications"].clone();
    assert_eq!(first["resendEveryBeats"], 5);
    let names: Vec<_> = first["channels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].clone())
        .collect();
    assert_eq!(names, [json!("Alpha"), json!("Bravo")]);
    assert_eq!(links(&pool, PING).await.len(), 2);

    let second = data(
        &schema,
        SET,
        json!({ "check": PING, "input": input(&json!([b]), None) }),
    )
    .await;
    let second = &second["setCheckNotifications"];
    assert_eq!(second["resendEveryBeats"], 0);
    assert_eq!(second["channels"].as_array().unwrap().len(), 1);
    assert_eq!(links(&pool, PING).await, [b.as_str().unwrap()]);
    assert_eq!(resend(&pool, PING).await, Some(0));

    let none = data(
        &schema,
        SET,
        json!({ "check": PING, "input": input(&json!([]), Some(1000)) }),
    )
    .await;
    assert_eq!(none["setCheckNotifications"]["channels"], json!([]));
    assert!(links(&pool, PING).await.is_empty());
    assert_eq!(
        resend(&pool, SSH_PORT).await,
        None,
        "another check is untouched"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn not_005_resend_beats_outside_zero_to_1000_are_refused(pool: PgPool) {
    let schema = schema_for(&pool);
    let a = create(&schema, webhook("Alpha")).await["id"].clone();
    for beats in [-1, 1001, 70_000, 4_294_967_297] {
        let response = set(&schema, PING, input(&json!([a]), Some(beats))).await;
        assert_eq!(
            validation_fields(&response).first().map(String::as_str),
            Some("resendEveryBeats"),
            "{beats}"
        );
    }
    assert!(links(&pool, PING).await.is_empty());
    assert_eq!(resend(&pool, PING).await, None);
    for beats in [0, 1000] {
        data(
            &schema,
            SET,
            json!({ "check": PING, "input": input(&json!([a]), Some(beats)) }),
        )
        .await;
        assert_eq!(resend(&pool, PING).await, Some(beats as i32));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn not_003_an_unknown_check_is_not_found_and_an_unknown_channel_is_validation(pool: PgPool) {
    let schema = schema_for(&pool);
    let a = create(&schema, webhook("Alpha")).await["id"].clone();
    for channels in [json!([a]), json!([])] {
        let response = set(&schema, NO_CHECK, input(&channels, Some(3))).await;
        assert_eq!(
            super::harness::error_json(&response)["extensions"]["code"],
            "NOT_FOUND"
        );
    }
    assert_eq!(resend(&pool, NO_CHECK).await, None);

    let response = set(&schema, PING, input(&json!([a, NO_CHANNEL]), Some(3))).await;
    assert_eq!(validation_fields(&response), ["channelIds"]);
    assert!(
        links(&pool, PING).await.is_empty(),
        "the whole change is refused"
    );
    assert_eq!(resend(&pool, PING).await, None);

    let response = set(&schema, PING, input(&json!(["not-an-id"]), None)).await;
    assert_eq!(validation_fields(&response), ["channelIds"]);
    let response = set(&schema, "nope", input(&json!([]), None)).await;
    assert_eq!(validation_fields(&response), ["checkId"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn log_005_a_change_writes_one_audit_entry_on_the_check_and_one_event(pool: PgPool) {
    let schema = schema_for(&pool);
    let a = create(&schema, webhook("Alpha")).await["id"].clone();
    sqlx::query("DELETE FROM events.audit_log")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM events.event_log")
        .execute(&pool)
        .await
        .unwrap();
    data(
        &schema,
        SET,
        json!({ "check": PING, "input": input(&json!([a]), Some(12)) }),
    )
    .await;
    set(&schema, NO_CHECK, input(&json!([]), None)).await;
    let audits: Vec<Value> = sqlx::query_scalar("SELECT to_jsonb(t) FROM events.audit_log t")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0]["action"], "update");
    assert_eq!(audits[0]["target_type"], "check");
    assert_eq!(audits[0]["target_id"], PING);
    assert_eq!(audits[0]["operator_id"], OPERATOR_ID);
    assert_eq!(
        audits[0]["change"],
        json!({ "channelIds": [a], "resendEveryBeats": 12 })
    );
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM events.event_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(events, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn api_003_a_read_key_cannot_set_check_notifications(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let reader = super::guard::api_key(&pool, 9, "read").await;
    let writer = super::guard::api_key(&pool, 10, "write").await;
    let variables = json!({ "check": PING, "input": input(&json!([]), Some(4)) });
    let refused = hub.post(Some(&reader), SET, variables.clone()).await;
    assert_eq!(
        error(&refused),
        (
            "This token may only read.".to_owned(),
            "FORBIDDEN".to_owned()
        )
    );
    assert_eq!(resend(&pool, PING).await, None);
    let none = hub.post(None, SET, variables.clone()).await;
    assert_eq!(error(&none).1, "UNAUTHENTICATED");
    let saved = hub.post(Some(&writer), SET, variables).await;
    assert_eq!(saved["errors"], Value::Null, "{saved}");
    assert_eq!(resend(&pool, PING).await, Some(4));
}
