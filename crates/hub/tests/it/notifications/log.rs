//! The delivery log (`Query.deliveries`) and the test notification (`sendTestNotification`)
//! through the API (NOT-004, API-003, API-006, LOG-005, LOG-007, P3-003).

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        events::Bus,
        notifications::graphql::{Mutation, Query, last_delivery_loader},
        targets::AddressPolicy,
    },
};
use async_graphql::{EmptySubscription, Request, Schema, Variables};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    guard::api_key,
    harness::{
        TestSchema, create, create_query, data, error_json, holds_no_secret, run, validation_fields,
    },
};
use crate::hub::{Hub, error, set_operator, start, test_vault};

const PAGE: &str = "query($channelId: ID, $first: Int, $after: String) { \
    deliveries(channelId: $channelId, first: $first, after: $after) { \
      edges { cursor node { id channelId channelName kind at ok reason code } } \
      pageInfo { hasNextPage endCursor } } }";

const SEND_TEST: &str = "mutation($id: ID!) { sendTestNotification(id: $id) { \
    id channelId channelName kind at ok reason code } }";

/// The schema with the hub's real address policy and a clock stopped at the current time, so
/// a delivery falls inside the partition window.
fn live_schema(pool: &PgPool) -> (TestSchema, DateTime<Utc>) {
    let now = Utc::now()
        .duration_trunc(TimeDelta::microseconds(1))
        .unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(Bus::new(16))
        .data(clock)
        .data(test_vault())
        .data(AddressPolicy::default())
        .data(last_delivery_loader(pool.clone()))
        .finish();
    (schema, now)
}

/// A webhook at a loopback address, which the hub's address policy refuses (P3-003).
fn loopback_hook(name: &str) -> Value {
    json!({
        "name": name,
        "settings": { "webhook": {
            "url": "https://127.0.0.1:9/hooks/s3cr3t-path",
            "method": "POST",
            "headers": [{ "name": "X-Api-Key", "value": "header-s3cr3t" }],
        } },
    })
}

async fn delivered(pool: &PgPool, channel: &Value, at: DateTime<Utc>) {
    let channel: Uuid = channel.as_str().unwrap().parse().unwrap();
    sqlx::query("SELECT notifications.ensure_day_partition($1)")
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO notifications.deliveries \
         (id, channel_id, channel_name, kind, at, ok, reason, code) \
         VALUES ($1, $2, 'Ops hook', 'down', $3, true, NULL, NULL)",
    )
    .bind(Uuid::now_v7())
    .bind(channel)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();
}

async fn page(schema: &TestSchema, variables: Value) -> Value {
    data(schema, PAGE, variables).await["deliveries"].clone()
}

fn times(page: &Value) -> Vec<DateTime<Utc>> {
    page["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["at"].as_str().unwrap().parse().unwrap())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_the_delivery_log_pages_newest_first(pool: PgPool) {
    let (schema, now) = live_schema(&pool);
    let first = create(&schema, loopback_hook("First")).await;
    let second = create(&schema, loopback_hook("Second")).await;
    let at = |minutes| now - TimeDelta::minutes(minutes);
    for minutes in [1, 3, 5] {
        delivered(&pool, &first["id"], at(minutes)).await;
    }
    delivered(&pool, &second["id"], at(2)).await;

    let one = page(&schema, json!({ "first": 2 })).await;
    let two = page(
        &schema,
        json!({ "first": 2, "after": one["pageInfo"]["endCursor"] }),
    )
    .await;
    let only_first = page(&schema, json!({ "channelId": first["id"] })).await;

    assert_eq!(times(&one), [at(1), at(2)]);
    assert_eq!(one["pageInfo"]["hasNextPage"], true);
    assert_eq!(
        one["pageInfo"]["endCursor"], one["edges"][1]["cursor"],
        "the end cursor is the last edge's"
    );
    assert_eq!(times(&two), [at(3), at(5)]);
    assert_eq!(two["pageInfo"]["hasNextPage"], false);
    assert_eq!(times(&only_first), [at(1), at(3), at(5)]);
    let node = &only_first["edges"][0]["node"];
    assert_eq!(node["channelId"], first["id"]);
    assert_eq!(
        (&node["kind"], &node["ok"], &node["reason"]),
        (&json!("DOWN"), &json!(true), &Value::Null)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_an_empty_log_is_an_empty_page(pool: PgPool) {
    let (schema, _) = live_schema(&pool);

    let empty = page(&schema, json!({})).await;

    assert_eq!(
        empty,
        json!({ "edges": [], "pageInfo": { "hasNextPage": false, "endCursor": null } })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_bad_page_arguments_are_named(pool: PgPool) {
    let (schema, _) = live_schema(&pool);
    for (variables, fields) in [
        (json!({ "first": 0 }), vec!["first"]),
        (json!({ "first": 201 }), vec!["first"]),
        (json!({ "after": "d1.12" }), vec!["after"]),
        (json!({ "channelId": "x" }), vec!["channelId"]),
        (
            json!({ "channelId": "x", "first": -1, "after": "" }),
            vec!["channelId", "first", "after"],
        ),
    ] {
        let response = run(&schema, PAGE, variables.clone()).await;
        assert_eq!(validation_fields(&response), fields, "{variables}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn p3_003_a_test_to_a_refused_address_is_logged_unsent_and_audited(pool: PgPool) {
    let (schema, now) = live_schema(&pool);
    let hook = create(&schema, loopback_hook("Ops hook")).await;

    let answer = data(&schema, SEND_TEST, json!({ "id": hook["id"] })).await;

    let delivery = &answer["sendTestNotification"];
    assert_eq!(delivery["channelId"], hook["id"]);
    assert_eq!(delivery["channelName"], "Ops hook");
    assert_eq!(delivery["kind"], "TEST");
    assert_eq!(delivery["ok"], false);
    assert_eq!(delivery["reason"], "TARGET_NOT_PERMITTED");
    assert_eq!(delivery["code"], Value::Null);
    assert_eq!(
        delivery["at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        now
    );
    holds_no_secret("the answer", &answer.to_string());
    let logged = page(&schema, json!({})).await;
    assert_eq!(logged["edges"][0]["node"], *delivery);
    let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, 0, "a test is never queued");
    let audits: Vec<(String, String, Value)> = sqlx::query_as(
        "SELECT action, target_type, change FROM events.audit_log WHERE action = 'send'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        audits,
        [(
            "send".to_owned(),
            "channel".to_owned(),
            json!({ "name": "Ops hook", "ok": false, "reason": "target_not_permitted", "code": null })
        )]
    );
    let events: Vec<String> = sqlx::query_scalar("SELECT message FROM events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    holds_no_secret("the events", &events.join("\n"));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_test_through_no_such_channel_is_not_found(pool: PgPool) {
    let (schema, _) = live_schema(&pool);

    let missing = run(
        &schema,
        SEND_TEST,
        json!({ "id": "0190a000-0000-7000-8fff-000000000001" }),
    )
    .await;
    let malformed = run(&schema, SEND_TEST, json!({ "id": "x" })).await;

    assert_eq!(error_json(&missing)["extensions"]["code"], "NOT_FOUND");
    assert_eq!(validation_fields(&malformed), ["id"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_the_log_and_the_test_need_an_operator(pool: PgPool) {
    let (schema, _) = live_schema(&pool);
    for (query, variables) in [
        (PAGE, json!({})),
        (
            SEND_TEST,
            json!({ "id": "0190a000-0000-7000-8fff-000000000001" }),
        ),
    ] {
        let request = Request::new(query).variables(Variables::from_json(variables));
        let response = schema.execute(request).await;
        assert_eq!(
            error_json(&response)["extensions"]["code"],
            "UNAUTHENTICATED",
            "{query}"
        );
    }
}

/// Through the hub as `main.rs` builds it: a read key reads the log but sends no test (R-100).
#[sqlx::test(migrations = "./migrations")]
async fn api_003_a_read_key_reads_the_log_but_sends_no_test(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let created = hub
        .post(
            Some(&token),
            &create_query(),
            json!({ "input": loopback_hook("Ops hook") }),
        )
        .await;
    let id = created["data"]["createNotificationChannel"]["id"].clone();
    let reader = api_key(&pool, 5, "read").await;

    let listed = hub.post(Some(&reader), PAGE, json!({})).await;
    let refused = hub
        .post(Some(&reader), SEND_TEST, json!({ "id": id }))
        .await;

    assert_eq!(listed["errors"], Value::Null, "{listed}");
    assert_eq!(listed["data"]["deliveries"]["edges"], json!([]));
    assert_eq!(
        error(&refused),
        (
            "This token may only read.".to_owned(),
            "FORBIDDEN".to_owned()
        )
    );
    let sent: i64 =
        sqlx::query_scalar("SELECT count(*) FROM events.audit_log WHERE action = 'send'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sent, 0);
}
